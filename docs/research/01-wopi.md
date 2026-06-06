# 01 — WOPI for Casual Drive

Research brief for a Rust/Axum WOPI host + retrofitting WOPI client support into the existing `sheet/` and `document/` editors. Primary source: Microsoft Cloud Storage Partner Program (CSPP) docs. Collabora / ONLYOFFICE specifics are noted as `[unverified]` where the primary URL was unreachable in this session and the claim rests on a WebSearch snippet.

## TL;DR

- Required host endpoints for real-time edit-and-save: **CheckFileInfo, GetFile, PutFile, Lock, Unlock, RefreshLock, UnlockAndRelock**. `GetLock` optional. `PutRelativeFile` only needed for Save-As.
- Locks: **30 min, one-per-file, opaque ≤1024 ASCII**. Lock-mismatch ⇒ `409` *with* `X-WOPI-Lock: <current>` header — that header is the discovery channel.
- Co-author = **one lock per session**. Office holds it with user A's token; PutFile rotates tokens (Excel always uses "principal user" = latest joiner). Host MUST accept Unlock/Refresh under any participating user's token.
- Discovery XML lives on the *client* at `/hosting/discovery`; host fetches and caches ~12–24h. `urlsrc` placeholders are substituted by the host; only `WOPI_SOURCE` is mandatory.
- Access tokens are host-opaque, scoped `(user, resource)`, sent as `?access_token=…`. `access_token_ttl` is **absolute JS-epoch ms**. Never auto-revoke.
- Proof keys = RSA-SHA256 over `[len|token][len|URL_UPPER][len|i64_ticks]`. Current+old keys, 3 accept combos, ts ≤20 min. Failure → HTTP 500.
- Host page POSTs token into a JS-created named iframe whose `action=` is the discovery action URL. Never GET.
- sheet/ already has ~500 LOC of "WOPI-on-self" scaffolding. document/'s `host.Integration` enumerates a `wopi` impl that's unwritten. Both editors need: discovery doc + iframe entry route + lock-refresh loop tied to room lifetime.

## 1. Minimum endpoint set

`rest/endpoints`: "All actions require the CheckFileInfo and GetFile operations." `online/discovery#action-requirements`: the `edit` action requires `update` (= PutFile + PutRelativeFile) and `locks` (= Lock + Unlock + RefreshLock + UnlockAndRelock).

| Operation | Verb + path | `X-WOPI-Override` | Required |
|---|---|---|---|
| CheckFileInfo | `GET /wopi/files/{id}` | — | yes |
| GetFile | `GET /wopi/files/{id}/contents` | — | yes |
| PutFile | `POST /wopi/files/{id}/contents` | `PUT` | yes |
| PutRelativeFile | `POST /wopi/files/{id}` | `PUT_RELATIVE` | yes (Save-As) |
| Lock | `POST /wopi/files/{id}` | `LOCK` | yes |
| Unlock | `POST /wopi/files/{id}` | `UNLOCK` | yes |
| RefreshLock | `POST /wopi/files/{id}` | `REFRESH_LOCK` | yes |
| UnlockAndRelock | `POST /wopi/files/{id}` | `LOCK` + `X-WOPI-OldLock` present | yes |
| GetLock | `POST /wopi/files/{id}` | `GET_LOCK` | optional (`SupportsGetLock`) |

URLs MUST start with `/wopi/` (no `/ids/`). Containers, ecosystem, bootstrapper, Delete/RenameFile, OneNote, broadcast, CSPP-Plus RTC: out of scope for v0.

Status-code contract: `200` success; `400` if `X-WOPI-Lock` missing; `401` bad token; `404` not-found/not-authorized; **`409` lock mismatch with `X-WOPI-Lock: <current>` response header**; `412` for GetFile over `X-WOPI-MaxExpectedSize`; `413` for PutFile over host cap; `500` server error.

`CheckFileInfo` required properties: **BaseFileName, OwnerId, Size, UserId, Version** (`Version` is string-typed even when numeric). To enable editing add `UserCanWrite=true`, `SupportsUpdate=true`, `SupportsLocks=true`, `SupportsExtendedLockLength=true`. Anonymous: set `IsAnonymousUser=true`; `UserId` may be omitted but `OwnerId` is still mandatory. Optional `FileUrl` gives Office a CDN bypass for GetFile but doesn't replace it. Omit unwanted properties — never send `null`.

## 2. Discovery XML

The WOPI *client* (Office for the web, Collabora, ONLYOFFICE, our editors) publishes `discovery.xml`. The host fetches and caches it; the host never publishes one.

Office for the web endpoints (`online/build-test-ship/environments`):

- Production: `https://onenote.officeapps.live.com/hosting/discovery`
- Dogfood: `https://ffc-onenote.officeapps.live.com/hosting/discovery`

Collabora and ONLYOFFICE expose the equivalent at `/hosting/discovery` on their respective servers, following the Microsoft schema [unverified — Collabora/ONLYOFFICE primary URLs were blocked by WebFetch in this session].

Shape: `<wopi-discovery>` > `<net-zone>` > `<app name="Word" favIconUrl="…">` > `<action name="edit" ext="docx" requires="locks,update" urlsrc="…"/>`. Example from `online/discovery`:

```xml
<action name="edit" ext="docx" requires="locks,update"
        urlsrc="https://word-edit.officeapps.live.com/we/wordeditorframe.aspx?
        <ui=UI_LLCC&><rs=DC_LLCC&><showpagestats=PERFSTATS&>"/>
```

`urlsrc` is a template. Host parses `<name=PLACEHOLDER&>` segments: known placeholder → substitute and drop the angle brackets; unknown → drop the whole segment. **`WOPI_SOURCE` is the one mandatory placeholder** (URL-encoded WOPISrc); others (`UI_LLCC`, `DC_LLCC`, `SESSION_CONTEXT`) are optional. `SESSION_CONTEXT` is echoed back on every subsequent request in `X-WOPI-SessionContext` — useful for log correlation.

The `<proof-key>` element carries the RSA public key in both `.NET CspBlob` form (`value` / `oldvalue` attributes) and portable RSA form (`modulus` / `exponent` / `oldmodulus` / `oldexponent`, base64).

Refresh cadence: Microsoft recommends 12–24h plus re-fetch immediately on proof-key validation failure (failure ≈ key rotation). **Do not honour the HTTP `Expires` header** on the discovery doc — explicitly broken per `online/discovery`.

## 3. Access tokens + proof keys

### Access tokens (`rest/concepts#access-token`)

Host-issued opaque; client never parses. Scoped to `(user, resource)` — never reused across users or files. Must match the CheckFileInfo permission bits. Sent on every request as `?access_token=…`; `Authorization: Bearer` is optional, so hosts MUST accept the URL param. `access_token_ttl` is **absolute JS-epoch milliseconds** (recommend ~10h); `0` means unknown and disables Office's save-prompt-before-expiry → data-loss risk. Don't auto-revoke — Office assumes validity until advertised expiry, and early revocation triggers session-timeout loops.

For Drive: signed JWT `{file_id, user_id, perms, exp}`. The `file_id` claim MUST be compared against URL `:id` on every request — sheet's `apps/server/src/wopi.ts:48-60` already does this; reuse the pattern.

### Proof keys (`online/scenarios/proofkeys`)

Office's defense against forged requests using leaked tokens. Office-for-the-web-specific; optional if Drive's only clients are our own editors, mandatory if we federate to Office.

Headers per request: `X-WOPI-Proof` (RSA-SHA256, current key), `X-WOPI-ProofOld` (same payload, previous key), `X-WOPI-TimeStamp` (`DateTime.UtcNow.Ticks`, 100-ns intervals since 0001-01-01, signed i64).

Signed byte sequence, big-endian:

```
[i32 len(token)][UTF-8 token]
[i32 len(URL)  ][UTF-8 URL_UPPERCASED, full querystring]
[i32 8         ][i64 timestamp]
```

Accept ANY of: (1) `Proof` verifies under current key; (2) `ProofOld` verifies under current key (client rotated after issuing); (3) `Proof` verifies under old key (host hasn't refreshed discovery). Reject if `TimeStamp` > 20 min old. Failure → **HTTP 500** (not 401, per spec). On success against old key, re-fetch discovery.

Rust: `rsa` crate, `RsaPublicKey::new(BigUint::from_bytes_be(&modulus), BigUint::from_bytes_be(&exponent))` then `key.verify(Pkcs1v15Sign::new::<Sha256>(), &expected, &sig)`. Port the proof-key fixtures from https://github.com/Microsoft/Office-Online-Test-Tools-and-Documentation as a unit test.

Gotcha: TLS terminated ahead of Axum makes the app see `http://` while Office signed `https://`. Preserve scheme via `X-Forwarded-Proto` and reconstruct, or terminate TLS in-process.

## 4. Lock semantics

From `rest/concepts#lock` and the per-operation pages:

- **One lock per file.** Lock ID is opaque ≤1024 ASCII (≤256 without `SupportsExtendedLockLength`). Host stores verbatim, never parses.
- **Auto-expires after 30 min** unless refreshed. Normative. Hosts must enforce.
- **Not user-bound.** "A WOPI host might receive a Lock call with an access token that belongs to User A. The host might later receive an Unlock call with an access token that belongs to User B. As long as User B has rights to edit the file, and the X-WOPI-Lock request header matches the lock ID, the Unlock request should be honored." (`rest/concepts#lock`)
- **`Lock` with the current lock ID = RefreshLock.** Per `rest/files/lock`: hosts "should treat the request as if it's a RefreshLock request."
- **`UnlockAndRelock` reuses `X-WOPI-Override: LOCK`.** Differentiated from Lock solely by the presence of `X-WOPI-OldLock`. Must be atomic — no observable unlocked state mid-op. A naive router that dispatches on Override alone will mis-route this.
- **`PutFile` on an unlocked file**: allowed iff the file is 0 bytes (the createnew case). Any other size → 409.
- **The 409 + `X-WOPI-Lock` response header is mandatory and asymmetric.** Hosts MUST emit `X-WOPI-Lock: <current>` on 409 (or empty string if currently unlocked, or omit if the current lock is non-WOPI-representable). The 200 path forbids the same header. Forgetting the 409 header sends Office into retry-spin or worse.

Observed refresh cadences: Office calls RefreshLock every ~10 min; Collabora's `storage.wopi.locking.refresh` defaults to 900 s = 15 min [grounded via WebSearch summary of Collabora SDK config docs]; ONLYOFFICE follows the spec's 30-min budget [unverified — primary api.onlyoffice.com page WebFetch-denied]. Host stale-detect should compare `lock_age > 30 min - grace` rather than picking a tight 10-min window, otherwise Collabora's lock will look stale to us prematurely.

Concurrent edits across *different* WOPI clients (Office + Collabora simultaneously on the same file) are NOT mediated by WOPI. Whoever locks first wins; the second sees 409 → "locked by other." Concurrent edits *within* a single WOPI client are that client's problem (§5).

## 5. Real-time co-editing on top of a per-file lock

**WOPI does not do co-editing. The WOPI client does.**

### Office for the web (source: `online/scenarios/coauth`)

User A opens → `CheckFileInfo` (A) → `Lock` (A, Office-internal lock ID). User B opens same doc → `CheckFileInfo` (B); if write perms confirmed, B *joins* the existing session — **no second Lock**. Edits merge inside Office's servers. PutFile fires periodically:

- Word: every 30 s if dirty, most-recent-editor's token.
- Excel: every 2 min, always the **principal user** (latest joiner).
- PowerPoint: 60 s if dirty (3 min single-user), most-recent-editor.

Perm re-checks: Word/PPT call CheckFileInfo per user every ≤5 min; Excel calls RefreshLock per user every ≤15 min. Lose perms mid-session → booted. Last user leaves → `Unlock`.

Implication: lock count is never a proxy for editor count (always 1 during co-edit). Host MUST accept Unlock/RefreshLock under any participating user's token if perms + lock-ID match. `X-WOPI-Editors` request header on PutFile is the audit channel, not the token-user.

### Collabora Online

Coolwsd's `DocumentBroker` owns the canonical document in a kit child process; users speak WebSocket to it; tile-streamed rendering. From the host's POV, Collabora is ONE WOPI client per file: one Lock, one debounced PutFile stream. PutFile fires on autosave + on last-user-disconnect. [unverified — Collabora SDK URLs blocked by WebFetch this session; sourced from DeepWiki summary of CollaboraOnline/online and the Collabora SDK config doc.]

### ONLYOFFICE Docs

Same shape: Document Server presents as one WOPI client per file. Ops: CheckFileInfo, GetFile, Lock, RefreshLock, Unlock, PutFile, RenameFile. Default 30-min lock, refreshed by the editor for the session's duration. Their native integration also layers a `callbackUrl` mechanism, orthogonal to WOPI mode. [unverified — primary api.onlyoffice.com pages WebFetch-denied; sourced from WebSearch snippets and DeepWiki ONLYOFFICE/DocumentServer/7.4.]

### Implication for Casual Drive

Honour exactly one lock per file; accept full-file binary overwrites (no diff/patch at the WOPI layer); bump `Version` + emit `X-WOPI-ItemVersion` on every successful PutFile; never assume the PutFile-token user is the only author.

## 6. What sheet/ and document/ must add

Both editors already have partial WOPI surface; the retrofit isn't from zero.

### sheet/ — current state

- `apps/server/src/wopi.ts` (293 LOC) — working WOPI *host* (CheckFileInfo, GetFile, PutFile) with JWT-scoped tokens; `file_id` claim already validated against URL `:id` (see lines 14-40, 48-60).
- `apps/web/src/file-source/wopi-file-source.ts` (215 LOC) — `FileSource` calling sheet's *own* WOPI routes for an embed bootmode. This is WOPI-as-self-hosted-file-IO, not a discovery-driven client of an external host.
- `playwright.wopi.config.ts` + `tests/e2e/wopi/wopi-embed-flow.spec.ts` — port-3066 e2e harness.

To become a WOPI editor client for Drive, sheet needs:

1. **`/hosting/discovery`** on Fastify, advertising `xlsx`/`ods`/`csv`/`tsv` for `edit` and `view` with `urlsrc` placeholders (`WOPI_SOURCE`, `UI_LLCC`, `SESSION_CONTEXT`).
2. **Iframe entry route** (e.g. `/wopi/editor`) consuming POSTed `access_token`/`access_token_ttl`, deriving `WOPISrc` from query, calling CheckFileInfo → GetFile → ExcelJS worker. Save path: existing autosave → `POST WOPISrc/contents` with `X-WOPI-Lock`, `X-WOPI-Override: PUT`.
3. **Lock loop**: `LOCK` with self-generated UUID on first edit; `RefreshLock` every 10 min; `UNLOCK` on `beforeunload` via `navigator.sendBeacon`. On 409, read `X-WOPI-Lock` from response and either surface "locked" UI or `UnlockAndRelock`.
4. **Co-edit topology**: sheet's existing Yjs/Hocuspocus runs *underneath* WOPI. Drive sees one WOPI client per file regardless of how many tabs join the Yjs room; the principal tab holds the lock and owns PutFile.
5. **Naming hygiene**: rename `wopi.ts` → `wopi-self-host.ts` (kept for desktop/embed), add `wopi-discovery.ts` + `apps/web/src/wopi-client/`. Conflating "I host WOPI" with "I am a WOPI client" will confuse forever.

### document/ — current state

- Stateless Go gateway (`backend/`); Yjs over WS; `host.Integration` interface in `backend/internal/host/` already enumerates `inline | wopi | jwtapi` (CLAUDE.md line 158, architecture block lines 22-32).
- `backend/test/mock-wopi/` exists but is **empty**.
- `HOST_INTEGRATION` env var already defined; wopi impl unwritten.

Needs:

1. **`backend/internal/host/wopi/`** — the missing concrete `host.Integration` impl. GetFile/PutFile against configured `WOPISrc`; lock acquisition + 10-min refresh ticker tied to room lifecycle. The "snapshot worker → host on room drain" hook (CLAUDE.md line 26) is where final PutFile + Unlock fire.
2. **`/hosting/discovery` + iframe entry route** in `docx-editor/`, mirroring sheet's, booting ProseMirror from GetFile bytes.
3. **Lock owned by the gateway, not the browser.** The gateway is the natural principal — it knows first-join and last-leave. Browser tabs manage Yjs presence only.
4. **Populate `backend/test/mock-wopi/`** with a Go test double covering all lock states (unlocked, locked-by-me, locked-by-other, stale, just-expired, UnlockAndRelock race).

### Common

POST tokens into the iframe, never GET; dynamically create the iframe via JS to defeat bfcache double-load (`online/hostpage`). Emit `X-WOPI-RequestingApplication: casual-sheet/0.2.x` etc. for log correlation. Cache discovery ≥12h; re-fetch on proof-key failure when Office federation lands.

## 7. Pain points and gotchas

- **409 + `X-WOPI-Lock` response header is mandatory and asymmetric** (forbidden on 200, required on 409).
- **`Authorization: Bearer` is optional;** query `access_token` is canonical. Demanding Bearer breaks Office.
- **`UnlockAndRelock` shares `X-WOPI-Override: LOCK` with Lock** — distinguished only by `X-WOPI-OldLock` presence.
- **Proof-key URL is uppercased including the full querystring**; don't re-encode, only uppercase the bytes.
- **TLS terminated ahead of the app breaks proof** (Office signed `https://`, app sees `http://`). Reconstruct via `X-Forwarded-Proto` or terminate TLS in-process.
- **`access_token_ttl` is absolute ms, not a duration**; the doc admits the name is wrong.
- **Don't revoke tokens early** — Office assumes validity until advertised expiry.
- **PutFile no-lock path only for 0-byte files** (createnew); every other PutFile needs `X-WOPI-Lock`.
- **`Version` must change on every PutFile and be `string`-typed**, even when numeric.
- **`OwnerId`/`UserId` must be alphanumeric** — strip UUID dashes.
- **Excel's PutFile-token user ≠ latest editor** (principal-user rule). Audit by `X-WOPI-Editors`.
- **Two different WOPI clients cannot co-edit the same file** — the lock is exclusive across clients.
- **Discovery `Expires` header lies.** Cache on your own schedule; re-fetch on proof-key failure.
- **WOPI Validator destroys the test file.** Use a throwaway `.wopitest` in CI.
- **Host page must JS-create the iframe** and POST into it; server-rendered `<iframe src=…>` breaks under bfcache with spurious "file locked" / "token expired" errors.

## 8. Alternatives rejected

**Signed-URL handoff** — Drive issues a presigned download URL; editor edits in memory; PUTs back to a presigned upload URL. ~50 LOC each side. Fails on no-lock (no co-edit safety), no version negotiation (lost updates), no mid-session permission re-check, no path to interop with Office/Collabora/ONLYOFFICE later. Fine for single-user viewer; insufficient for multi-user editing.

**postMessage-only iframe with bespoke protocol** — Drive embeds the editor; they exchange JSON over `postMessage` for everything. Fails on: every op is custom and unverifiable, third-party editors can't integrate, no proof-key story so the embedded editor has to trust host-page origin alone (XSS in host = unrestricted file write), and we'd reinvent CheckFileInfo/Lock/PutFile from scratch. WOPI already uses postMessage as a UI-glue layer (`UI_Sharing`, `UI_Close`, `Edit_Notification` per `online/scenarios/postmessage`) on top of REST — that's the right division.

**WOPI wins** because (a) real spec + Microsoft-published validator + multiple correct sample impls; (b) free path to federate Office / Collabora / ONLYOFFICE later; (c) battle-tested lock/version semantics; (d) sheet/ already has 500 LOC of partial WOPI scaffolding to repurpose.

## Sources

Primary (Microsoft Learn — fetched this session):

- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/concepts
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/endpoints
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/common-headers
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/checkfileinfo
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/checkfileinfo/checkfileinfo-response
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/getfile
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/putfile
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/lock
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/unlock
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/refreshlock
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/unlockandrelock
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/getlock
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/discovery
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/scenarios/proofkeys
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/scenarios/coauth
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/scenarios/postmessage
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/hostpage
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/build-test-ship/environments
- https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/build-test-ship/validator

Secondary (WebSearch snippets — primary URLs WebFetch-denied this session, marked `[unverified]` in body):

- https://sdk.collaboraonline.com/docs/installation/Configuration.html — Collabora `storage.wopi.locking.refresh` default 900s.
- https://www.collaboraonline.com/blog/wopi-is-open-your-office-stack-should-be-too/
- https://deepwiki.com/CollaboraOnline/online/2.1-coolwsd-main-process — COOLWSD/DocumentBroker architecture summary.
- https://deepwiki.com/ONLYOFFICE/DocumentServer/7.4-wopi-protocol
- https://api.onlyoffice.com/docs/docs-api/more-information/faq/using-wopi/ — 30-min lock + refresh.
- https://www.mckennaconsultants.com/wopi-coauthoring-a-technical-guide-to-real-time-multi-user-document-editing/ — cross-check.
- https://github.com/Microsoft/Office-Online-Test-Tools-and-Documentation — proof-key fixtures + SampleHostPage.html.
- https://github.com/Microsoft/wopi-validator-core — open-source Validator.

Internal:

- /Users/sachin/Desktop/melp/services/sheet/CLAUDE.md
- /Users/sachin/Desktop/melp/services/sheet/README.md
- /Users/sachin/Desktop/melp/services/sheet/apps/server/src/wopi.ts (293-LOC WOPI host already implemented)
- /Users/sachin/Desktop/melp/services/sheet/apps/web/src/file-source/wopi-file-source.ts (215-LOC self-targeting client)
- /Users/sachin/Desktop/melp/services/sheet/playwright.wopi.config.ts
- /Users/sachin/Desktop/melp/services/document/CLAUDE.md
- /Users/sachin/Desktop/melp/services/document/README.md
- /Users/sachin/Desktop/melp/services/document/backend/test/mock-wopi/ (empty placeholder)
