# Spike #2 — WOPI host

Location: [`../../spikes/02-wopi-host/`](../../spikes/02-wopi-host/). Standalone Cargo project.

## Goal

Confirm the seven WOPI host endpoints from [`../research/01-wopi.md`](../research/01-wopi.md) §1 implement cleanly in Axum 0.8 — especially the spec's two trickiest contracts:

- **The asymmetric 409 + `X-WOPI-Lock` response header** (mandatory on 409, forbidden on 200).
- **`UnlockAndRelock` shares `X-WOPI-Override: LOCK`** with Lock, distinguished only by `X-WOPI-OldLock` presence.

Verified with an integration test that walks the full edit cycle plus the edge cases that bit other WOPI implementers.

## Outcome

Green. 8/8 integration tests pass.

```
test result: ok. 8 passed; 0 failed
```

| Test | What it proves |
|---|---|
| `happy_path_full_edit_cycle` | CheckFileInfo → GetFile → Lock → PutFile → RefreshLock → Unlock all 200, `X-WOPI-ItemVersion` bumps |
| `putfile_without_lock_returns_409_with_lock_header` | The mandatory + asymmetric 409 contract |
| `happy_putfile_does_not_send_lock_header_back` | The asymmetric *other half* — no `X-WOPI-Lock` on 200 |
| `createnew_zero_byte_put_without_lock_allowed` | Only PutFile-without-lock case the spec allows |
| `token_for_other_file_rejected` | File-id in token must match URL — basic auth scoping |
| `read_token_cannot_putfile` | Perm enforcement: Read claim → PutFile rejected |
| `unlock_and_relock_atomic_swap` | LOCK + `X-WOPI-OldLock` dispatches to UnlockAndRelock; new lock active afterwards |
| `lock_with_same_id_acts_as_refresh` | Per spec §4: "Lock with current lock ID = RefreshLock" |

## What worked

- **Axum 0.8 path syntax `{file_id}`** maps cleanly to `Path<String>`. Both `GET` and `POST` register on the same route via `.get(check_file_info).post(lock_dispatch)`.
- **The `lock_dispatch` switch on `X-WOPI-Override`** is six lines and handles all four lock-family operations cleanly. The "LOCK with `X-WOPI-OldLock` is actually UnlockAndRelock" rule is one `match` arm. No router-level surprise.
- **The 409 + header pattern** lives inside `WopiError::LockConflict(String)` with a custom `IntoResponse` impl. The single error type carries the current-lock string and renders it as `X-WOPI-Lock`. Callers just `Err(WopiError::LockConflict(current))` and the right wire shape happens automatically.
- **The 200-must-omit-`X-WOPI-Lock`-header** half just works because the success branches construct empty response heads. `happy_putfile_does_not_send_lock_header_back` is the regression test that locks this in.
- **JWT access tokens via `jsonwebtoken` 10.4** with HS256 + `aws_lc_rs` crypto provider. Mint and verify are ~30 LoC total. `WopiClaims { user_id, file_id, perms, exp, jti }` is the exact shape from [`../ARCHITECTURE.md §"Three-token identity model"`](../ARCHITECTURE.md).
- **Per-call file-id scoping** (`token.file_id == URL :file_id`) verified by `token_for_other_file_rejected`. This is the single most important auth check in the whole host.

## What surprised

1. **`jsonwebtoken` 10.x requires a crypto-provider feature.** Default build compiles, then panics at first sign/verify with "Could not automatically determine the process-level CryptoProvider". Fix: `features = ["aws_lc_rs"]` (or `rust_crypto`). The rust-stack brief already called out `aws_lc_rs` — confirmed correct.

2. **`HeaderName::from_static` requires a `const` context.** Defining `const H_LOCK: HeaderName = HeaderName::from_static("x-wopi-lock");` at module scope is the clean way; trying to do it inside a function panics at runtime if the string isn't validated. Trivial once you know.

3. **`CheckFileInfo` needed `Deserialize` for the integration test** to parse the JSON it serialised. Stop instinctively `#[derive(Serialize)]`-only on response types — round-trip via `from_slice` in tests catches the omission. Cheap reminder.

4. **`tower::ServiceExt::oneshot` works as advertised** even with state: `app.clone().oneshot(req)` against an `Arc<Mutex<...>>`-backed state lets every test build its own state, mutate it, and run isolated assertions. Phase 1 reuses this pattern verbatim.

## What's out of this spike (and where it goes)

| Out | Where |
|---|---|
| Discovery XML (`/hosting/discovery`) | Sheet/document, not Drive. Spike #3 in sheet/. |
| Proof-key RSA validation | Phase 3 (only when MS365 federation is enabled). The hook lives in `verify_token`; today it's `Ok(())`. |
| `PutRelativeFile` (Save-As) | Phase 1+ (route returns 501). |
| `GetLock` | Deferred; we don't advertise `SupportsGetLock`. |
| Lock persistence | Spike uses `Arc<Mutex<HashMap<...>>>`. Phase 1 uses `wopi_locks` table. |
| Proper streaming `PutFile` body | Spike buffers `Bytes`. Phase 1 streams via `Storage::put_stream`. |
| TLS / `X-Forwarded-Proto` reconstruction for proof | Deferred with proof-key. |
| `X-WOPI-Editors` audit channel | Phase 2 (audit-log). |

## Recommended revisions to ARCHITECTURE.md / CLAUDE.md before Phase 1

- **Pin `jsonwebtoken` 10.4+ with `aws_lc_rs` feature** in the starter Cargo.toml. Already in the rust-stack brief; promote to ARCHITECTURE.md §"Configuration" so it's not lost.
- **The `lock_dispatch` switch belongs in `drive-wopi`** as a public function, not buried in handler code. The "Override-then-OldLock" branch is the spec's most-misread rule; isolate it so it gets its own focused unit tests.
- **Define `WopiError` in `drive-wopi` with the `IntoResponse` impl** in this spike. The error type IS the response contract for half the spec; one place, one impl.

## Files

- [`spikes/02-wopi-host/Cargo.toml`](../../spikes/02-wopi-host/Cargo.toml) — axum 0.8, jsonwebtoken 10 (`aws_lc_rs`), minimum deps
- [`spikes/02-wopi-host/src/lib.rs`](../../spikes/02-wopi-host/src/lib.rs) — `AppState`, `WopiClaims`, `CheckFileInfo`, the 7-endpoint router, error → status mapping
- [`spikes/02-wopi-host/tests/edit_cycle.rs`](../../spikes/02-wopi-host/tests/edit_cycle.rs) — 8 integration tests against an in-memory `AppState`

## Decision

**Greenlit.** The WOPI host shape from ARCHITECTURE.md survives contact with the spec edge cases. The `WopiError::LockConflict(String)` pattern + `IntoResponse` impl is the right encoding of the 409 + header contract; carry it into `crates/drive-wopi`. Move to Spike #4 (two-origin Axum) next — Spike #3 (sheet/ retrofit) is cross-repo and gets staged separately.
