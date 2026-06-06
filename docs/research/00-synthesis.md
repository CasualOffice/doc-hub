# 00 — Research Synthesis

Distillation of briefs 01–06. What's locked, what's open, what changed from the initial framing.

## Locked decisions (sourced from briefs)

| Area | Decision | Source |
|---|---|---|
| Backend framework | **Axum 0.8** | 05 §1 |
| Storage abstraction | **OpenDAL** behind a thin `Storage` facade (~80–150 LoC) | 03 §9 |
| Storage backends shipped v0 | filesystem, memory, S3, MinIO — all first-class in `opendal::services` | 03 §1 |
| Editor handoff | **WOPI** (Drive = host; sheet, document = clients) | 01 |
| Auth model | **Single-tenant self-host** — one env-seeded admin + share-links | 02 §7 |
| Session layer | `tower-sessions`, server-side, `__Host-` cookie | 02 §3, 05 §4 |
| Password hash | `argon2id`, OWASP minimum `m=19 MiB, t=2, p=1` | 02 §3, 06 §7 |
| Cookie attrs | `__Host-` prefix, `HttpOnly`, `Secure`, **`SameSite=Lax`** (Strict breaks WOPI redirects) | 06 §7 |
| CSRF | session-bound double-submit token + Origin/Referer check on cookie-auth POST/PUT/DELETE | 02 §3, 06 §7 |
| Rate limiting | `tower_governor` (GCRA) — `/login` 10/min/IP, upload 30/min/IP, download 300/min/IP | 02 §3, 06 §5 |
| WOPI access token | HMAC-SHA256 over `{user_id, file_id, perms, exp, jti}`, **10 min TTL** (interactive sessions), validated per call | 02 §4, 06 §8 |
| WOPI lock duration | **30 min** per spec, refresh every ~10 min from client | 01 TL;DR, 01 §4 |
| WOPI proof-keys | Hook present; mandatory only if v0 federates to MS365 — **deferred** unless we explicitly opt in | 01 §3, 06 §8 |
| Signed URLs (fs/memory) | HMAC-SHA256 over `(key, exp, method)`, served via `/raw/{token}` on user-content origin, 5 min TTL, constant-time verify | 03 §5, 06 §9 |
| **Two-origin model** | App origin `drive.<host>` vs user-content origin `usercontent-drive.<host>`. **Boot refuses prod if they match.** | 06 §4 (non-negotiable #1) |
| Frontend stack | Radix Primitives + shadcn/ui + Motion + cmdk + vaul + sonner + Lucide on Inter | 04 §18, §Libraries |
| Icon family | **Lucide** (1,800+, 24×24, 2 px stroke), Phosphor as fallback | 04 §7 |
| Motion | `cubic-bezier(0.32, 0.72, 0, 1)` ease-out default; durations 80–250 ms UI, 400–600 ms transitions; springs for direct manipulation; honour `prefers-reduced-motion` everywhere | 04 §5 |
| Build & deploy | `rust-embed` SPA in single static binary, `cargo-chef` multi-stage Dockerfile, `debian-slim` runtime | 05 §10–11 |
| CI security gates | `cargo audit --deny warnings` + `cargo deny check` on every PR | 06 §12 |
| Dependency policy | All deps pinned (`Cargo.lock` committed), licence allowlist (`Apache-2.0`, `MIT`, `BSD-3-Clause`, `ISC`) | 06 §12 |

## Cross-brief tensions and resolutions

| # | Tension | Resolution |
|---|---|---|
| 1 | 03-storage recommends **OpenDAL**; 05-rust-stack §5/§12 sketches a hand-rolled trait with `aws-sdk-s3` | **OpenDAL wins.** 03's reasoning is stronger: capability gaps, retry/tracing layers, all 4 backends first-class, Apache TLP governance. Drop `aws-sdk-s3` from the starter `Cargo.toml`; replace `storage/{fs,memory,s3}.rs` with one `storage/mod.rs` wrapping `opendal::Operator`. |
| 2 | 02-auth references `axum-login`; 05-rust-stack §4 says skip it | **Skip `axum-login` for v0.** `tower-sessions` + ~30 LoC custom extractor is the dominant pattern in 2026. Add `axum-login` only if friction emerges. |
| 3 | 02-auth wants `SameSite=Strict`; 06-security wants `SameSite=Lax` | **`SameSite=Lax`.** WOPI uses cross-site editor → Drive redirects that Strict would block. Lax + CSRF token + Origin check is the standard belt-and-braces. |
| 4 | 04-polish says SPA serves from same origin as Drive app; 06-security mandates two origins | No real tension. **App SPA and Drive API both on `app_origin`**; only raw user bytes go to `usercontent_origin`. The SPA fetches metadata from app origin and redirects to user-content origin for downloads. |
| 5 | 05-rust-stack §5 wonders if AFIT works for `dyn Storage`; 03-storage §3 confirms native AFIT is **not** object-safe in 2026 | **Use `#[async_trait]` + `Arc<dyn Storage>`.** One heap alloc per call, irrelevant against I/O. |

## Surprises that changed the plan

1. **sheet/ already ships ~500 LOC of WOPI scaffolding** (`apps/server/src/wopi.ts` 293 LOC host + `apps/web/src/file-source/wopi-file-source.ts` 215 LOC client + `playwright.wopi.config.ts` e2e on port 3066). It's WOPI-as-self-host (not yet discovery-driven), but the JWT-validation + lock-routing patterns transfer directly. WOPI retrofit in sheet ≈ adding `/hosting/discovery` + iframe entry route + lock-refresh loop. **Not** a from-zero build.
2. **document/'s `host.Integration` already enumerates a `wopi` impl** that's unwritten (`backend/test/mock-wopi/` is an empty placeholder; `HOST_INTEGRATION` env var defined). The integration slot is reserved; we just fill it.
3. **WOPI lock duration is 30 min, not 30 s.** Refresh every ~10 min from client.
4. **Proof-key validation is optional unless we federate to MS365.** If Drive's only WOPI clients are sheet/ + document/, we can defer proof-key cryptography to a later phase. Big complexity win for v0.
5. **`Authorization: Bearer` is optional in WOPI;** query `access_token` is canonical. Mandating Bearer breaks Office.
6. **No production-grade Rust WOPI host crate exists.** `beatgammit/wopi-rs` last touched 2017. We build `drive-wopi` from spec; .NET `petrsvihlik/WopiHost` is the behavioural reference.

## Open user questions

Resolved on 2026-06-06:

| # | Question | Decision |
|---|---|---|
| 1 | MS365 (Office Online) federation in v0? | **No** — our own editors only. Proof-key RSA crypto deferred; the hook stays in the design for later. |
| 2 | Casual Slides — does it exist? | **Planned but not built.** Drive's MIME registry and config reserve the `.pptx` slot but no editor handoff is wired in v0. |
| 3 | Macro-enabled Office (`.xlsm`/`.docm`/`.pptm`)? | **Accept upload, refuse open-in-editor.** Stored as opaque blobs; user downloads them to run locally if they want. |
| 4 | Metadata DB — SQLite or Postgres? | **Both.** SQLite is the default and only required engine for v0. Postgres support is targeted for production deploys. Implication: every migration must be portable across both engines, and CI runs the test matrix on both. Costlier than picking one; user accepted the cost. |

Still open (does **not** block step 2):

- **Domain operational shape** — when migrating to `casualoffice.org`, the two-origin split (e.g. `drive.casualoffice.org` + `usercontent-drive.casualoffice.org`) needs to be confirmed and reflected in config + reverse-proxy examples. Park until deployment-config phase.

## What's next

Steps 2–6 of the original planning order:

2. **UX flows** — first-run, upload, open-in-editor, share-link create/revoke, rename, move, delete/trash/restore, search, multi-select, settings. Numbered scenarios before any pixels.
3. **UI surface spec** — sidebar, main view (list/grid), preview pane, command palette, modals, empty states. Built from the §04 token set + Lucide + the 10 commandments.
4. **Adapter contract** — formalise `trait Storage`'s public API (the OpenDAL facade) and write the conformance test sketch.
5. **WOPI integration protocol** — concrete sequence diagrams for "open a file from Drive" through both sheet/ and document/, including the discovery doc shape and the iframe POST mechanic.
6. **Phased milestone plan** — Phase 0 spikes → Phase 1 walking skeleton → Phase 2 collab/share polish → Phase 3 MS365 federation (if chosen).

Recommend tackling them in that order. Step 2 needs you in the loop; steps 4–5 are mostly mechanical and I can draft alone for your review.
