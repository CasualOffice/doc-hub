# Design: share `/raw` serves decrypted content (review finding #9)

Status: **implemented** — file-capability + proxy-decrypt approach, read-only for
legacy files (both decisions confirmed). See `crates/dochub-http/src/raw.rs`
(`serve_share`), `share.rs` (`download_share`), `dochub-storage`
(`mint_get_token`), `dochub-db/registry.rs` (`read_head_readonly`), and
`tests/share.rs::share_download_serves_decrypted_plaintext`.
Scope: fix the last confirmed adversarial-review finding. Security-sensitive; touches the two-origin model and the crypto boundary, so it gets a design gate.

## Problem

A share-link download served from the cookieless user-content origin returns the
**wrong bytes**:

- `raw::raw_get` (`crates/dochub-http/src/raw.rs`) reads via
  `state.storage.get(&key, None)` — the **low-level plaintext read, no
  decryption** — using a signed token whose key is the **legacy plaintext key
  `files/{id}`** (`share::download_share` → `storage.signed_get(storage_key(id))`).
- Every document written since the version-chain cutover is **AES-256-GCM
  ciphertext at rest**, content-addressed under `versions/{hash}`. So `/raw`
  streams undecryptable ciphertext for any modern file; it "works" only for
  never-re-saved legacy blobs still sitting at `files/{id}`.
- On the S3/MinIO backend it is worse: `signed_get` returns a **native
  presigned bucket URL**, redirecting the recipient straight to object storage —
  which holds ciphertext and has no way to decrypt. **Presign-to-bucket is
  fundamentally incompatible with at-rest encryption.**

Net: share downloads are broken for encrypted content, and the one path that
appears to work (legacy plaintext) is exactly the data we're retiring.

## Constraints (from CLAUDE.md — inviolable)

- **Server holds keys by design** (not zero-knowledge): server-side decryption
  in a handler is allowed and expected.
- **Two-origin model**: the user-content origin is cookieless, `CSP: sandbox;
  default-src 'none'`, serves only `/raw/{token}`. No session may authorize it —
  the token must be the sole capability.
- **No plaintext to storage**; decryption goes through `dochub-crypto`/`Storage`.
- **Tokens: distinct purposes, constant-time.** The signed-URL token is an HMAC
  over `(identifier, exp, method)`.

## Key realization

Because object storage holds only ciphertext and only the app has the
per-workspace DEK, **a share download must be proxied and decrypted by the app**
— it can never be a bucket-direct presigned URL. This is forced, not a
preference: it removes the native-presign branch from the share path entirely.

## Proposed design

Keep the existing shape — app origin gates (password/expiry/permission) then
redirects to `/raw/{token}` on the user-content origin — and fix what the token
authorizes and how `/raw` reads.

### 1. A share capability token, not a storage-key token

`download_share`, after all gates pass, mints a short-lived (120 s) HMAC token
whose identifier is the **file**, not a storage key:

```
payload = "GET\nshare:{file_id}\n{expiry_unix}"
token   = base64url(payload ‖ HMAC-SHA256(sign_key, payload))
```

This reuses the existing `mint_token`/`verify_token` machinery (the identifier
is just a string); the `share:` prefix distinguishes a capability token from a
legacy raw storage-key token. The token is minted **only after** the share gate,
so the cookieless `/raw` needs no further authz — the capability is the proof.
Always mint this token; **never** call `signed_get` for shares (that is what
introduced the native-presign branch).

### 2. `/raw` decrypts server-side, reusing the authenticated read path

`raw_get` verifies the token (HMAC + expiry, constant-time), then on a `share:`
identifier:

1. `FileRepo::find_by_id(file_id)` → the file (404 on miss).
2. Resolve the workspace: `file.workspace_id`.
3. Decrypt + stream the head version's plaintext through the **same** code the
   authenticated `download_file` uses — `read_document_bytes` /
   `Registry::read_or_backfill(workspace, file_id, …)` — which resolves the head
   version blob, gets the DEK (`WorkspaceDeks::get_or_create`), and
   `Storage::get_blob(dek, key)` (decrypts via `open()`).

Reusing that path is deliberate: **one decrypt-and-serve implementation**, no
second copy of crypto handling (secure-coding rule 4). A legacy file with no
version row is handled exactly as the authed path handles it today.

Legacy non-`share:` tokens (bare storage keys) keep the old `storage.get` path
for backward compatibility during rollout, then are removed once no minter emits
them.

### 3. Response headers unchanged

`/raw` keeps `Content-Disposition: attachment` + `nosniff` + the sandbox CSP.
Content-type comes from the file row. (A later, separate change could allow
`inline` preview for safe types; out of scope here.)

## Security analysis

- **Capability scope**: token authorizes exactly one file's plaintext for 120 s,
  minted only after password/expiry/permission checks. Narrow and short.
- **Constant-time**: HMAC verify is constant-time; share-token DB compare already
  is. No new comparison surface.
- **Origin isolation intact**: `/raw` still cookieless; the token, not a session,
  authorizes. Server-side decryption is permitted (keys are server-held).
- **No plaintext at rest**: bytes are decrypted in-process and streamed; nothing
  new is written.
- **Revocation/expiry**: the capability token is short-lived; the underlying
  share row's expiry/password are enforced at mint time. (A stolen 120 s token is
  the same exposure as today's signed URL — unchanged.)
- **Read-only**: the share path must not commit versions or mutate history as a
  side effect of a recipient's download (see open question 1).

## Alternatives considered

- **Token carries `versions/{hash}` storage key; `/raw` looks up workspace by
  key.** Spec-closest ("HMAC over key"), gives snapshot stability, but needs a
  `file_versions.storage_key` index (new migration) and an extra join, and does
  not reuse the authed read path as cleanly. Rejected for the file-capability
  approach unless snapshot-at-mint semantics are explicitly wanted.
- **Native presigned bucket URL.** Impossible with at-rest encryption (bucket
  holds ciphertext). Explicitly ruled out.
- **Decrypt into a temp plaintext object and presign that.** Writes plaintext to
  storage — violates the no-plaintext rule. Rejected.

## Open questions (need a decision)

1. **Legacy file with no version row on the share path**: backfill-and-commit v1
   (as the share creator) like the authed path, or a read-only decrypt that
   serves plaintext without writing history? Recommendation: **read-only** — a
   recipient's download should not mutate history.
2. **Snapshot vs. head**: serve the file's current head at redemption
   (file-capability) or the exact version at mint time (storage-key token)?
   Recommendation: **head** — a share link reflecting the live document is the
   expected behavior, and TTL is only 120 s anyway.

## Test plan

- End-to-end byte round-trip: upload → create share → `GET /raw/{token}` returns
  bytes **equal to the original plaintext** (the assertion the current tests are
  missing — they only check status codes).
- Encrypted-at-rest proof: the object at the version key is **not** equal to the
  plaintext, but `/raw` output **is** (decryption actually happens).
- Gate ordering: password/expiry rejected before any token is minted; expired
  capability token → 404/410 at `/raw`.
- Backend parity: the share path never emits a native presigned URL (always
  routes through `/raw`).

## Rollout

Single PR: token minting change in `share.rs`, `raw.rs` share-decrypt branch,
reuse of the registry read path, tests. No schema migration required under the
file-capability approach. Back-compat retained for legacy bare-key tokens.
