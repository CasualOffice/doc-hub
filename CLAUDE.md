# CLAUDE.md — instructions for Claude Code in this repo

## What this project is

**Casual Drive** — a self-hostable, file-centric Drive that opens `.xlsx` and `.docx` (and later `.pptx`) in the sibling Casual editors via the WOPI protocol. Part of the Casual Office suite (`schnsrw.live`; planned `casualoffice.org`).

Single Rust binary, two HTTP origins, four pluggable storage backends, single-tenant admin auth, polished web UI.

## Five inviolable rules

Set by the user. Never broken.

1. **Research first.** Investigate prior art before proposing. Briefs live in `docs/research/`.
2. **Plan UX.** Numbered flows before pixels. Spec lives in `docs/ux/01-flows.md`.
3. **Consistent UI.** Surfaces, copy, motion all coherent. Spec lives in `docs/ux/02-surface.md`.
4. **Industry-standard secure coding.** No homebrew crypto/auth. OWASP-aware. Checklist in `docs/research/06-security.md`.
5. **Polished, minimalistic design.** macOS-*app*-grade polish — Things 3 / Linear / Raycast quality bar, NOT a Finder clone. Tokens + 10 commandments in `docs/research/04-polish-principles.md`.

Default working mode: **plan → present → ask → code.** Do not skip the planning loop even for "simple" tasks.

## Stack (locked)

- **Backend:** Rust + Axum 0.8 + tokio + tower
- **Storage:** OpenDAL behind a thin `Storage` facade (fs / memory / S3 / MinIO — all four shipped v0)
- **Auth (v0):** single-tenant admin, `tower-sessions` + `__Host-cd_sid`, Argon2id passwords
- **Editor handoff:** WOPI (Drive = host; sheet/, document/ = clients)
- **Frontend:** React + Vite + Radix Primitives + shadcn/ui + Motion + cmdk + vaul + sonner + Lucide + Inter
- **DB:** SQLite default, Postgres for production (every migration must be portable across both)
- **SPA delivery:** `rust-embed`, single static binary
- **Docker:** `cargo-chef` multi-stage, `debian:trixie-slim` runtime
- **CI gates:** `cargo audit --deny warnings` + `cargo deny check` on every PR

## What's in scope (v0)

- Single-tenant admin signs in, manages files.
- Upload/download (single + selection-as-zip).
- Browse, rename, create folder, move (drag + picker), trash, restore from trash.
- Open `.xlsx` in sheet/ and `.docx` in document/ via WOPI.
- Search (Cmd-K palette).
- Share-links with optional password and expiry (Phase 2).
- Two-origin model: `drive.<host>` for the app; `usercontent-drive.<host>` for raw bytes.
- Light + dark themes from day one.
- Self-hostable in one Docker container on a $5 VPS.

## What's out of scope

- **MS365 / Office Online federation.** Proof-key RSA validation deferred; the hook stays in the design.
- **Multi-user accounts.** Single-tenant v0; OIDC slot ready for Phase 3.
- **Casual Slides handoff.** MIME slot reserved; no wiring until Slides exists.
- **Macro-enabled Office formats** (`.xlsm` / `.docm` / `.pptm`) — accepted as opaque blobs, never auto-opened in editor.
- **Auto-thumbnailing.** Decoding untrusted images is real CVE surface; needs sandboxed worker (Phase 3).
- **Sync clients.** Browser-only.
- **Real-time presence at the Drive level.** Editors carry their own collab; Drive's job is files.
- **A native AppKit/Catalyst app.** Tauri wrapping is the Casual Desktop lane.

## Required reading before substantive work

1. [`PLAN.md`](./PLAN.md) — phased delivery plan; know which phase we're in.
2. [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) — workspace, storage facade, WOPI sequence, two-origin model, three-token identity model.
3. [`docs/research/00-synthesis.md`](./docs/research/00-synthesis.md) — locked decisions + cross-brief tension resolutions.
4. [`docs/ux/01-flows.md`](./docs/ux/01-flows.md) — the 16 v0 user flows.
5. [`docs/ux/02-surface.md`](./docs/ux/02-surface.md) — visual surface spec, ASCII layouts, state checklists.

Then dip into the topic briefs (`docs/research/01-06.md`) when their domain comes up.

## Hard rules

### Storage goes through the facade — always

- Handler code talks to `Arc<Storage>`. Never reaches into `opendal::Operator` directly. The facade exists so the capability gates and `/raw/{token}` fallback work uniformly across backends.
- New backends added by listing a new `opendal::services::*` builder in `Storage::from_env`. The trait doesn't grow.

### WOPI conformance is non-negotiable

- The 7 endpoints, the status-code contract (especially the 409 + `X-WOPI-Lock` response header — mandatory on 409, forbidden on 200), and the lock semantics from `docs/research/01-wopi.md` §1 + §4 are spec. Don't improvise.
- Access tokens carry `(user_id, file_id, perms, exp, jti)` and are HMAC-SHA256. Validated server-side every call. File-id in URL MUST match file-id in token claim.
- Lock duration is **30 min** (not 30 s). Client refreshes every ~10 min. Stale locks (`expires_at < now()`) are treated as absent.
- Proof-key cryptography is NOT in v0 (no MS365 federation). The hook is present; the validation call is `Ok(())`. When/if MS365 federation lands, that hook becomes real.

### Two-origin model is non-negotiable

- App origin (`drive.<host>`) serves SPA, JSON API, WOPI endpoints. Strict CSP. Cookies live here.
- User-content origin (`usercontent-drive.<host>`) serves `/raw/{token}` only. `CSP: sandbox; default-src 'none'`. No cookies. `Content-Disposition: attachment` for non-previewable types.
- Boot **refuses to start in production** if `app_origin == usercontent_origin`. Test this.
- Do not weaken either CSP. Do not move `/raw/{token}` to the app origin. Do not set session cookies on the user-content origin.

### Three tokens, three purposes, never confuse them

- **Session cookie** (`__Host-cd_sid`): the admin's browser session. Server-side store.
- **WOPI access token**: per-launch, per-file, 10-min TTL. HMAC-signed claim. Sent on every WOPI request.
- **Share-link token**: per-share-link row. Path segment `/s/<token>`. Verified by constant-time compare against the DB row.
- **(plus) Signed-URL token**: fs/mem only, for the `/raw/{token}` handler. HMAC over `(key, exp, method)`.

Don't reuse one for another's job.

### Polish bar is enforceable

The 10 commandments from `docs/research/04-polish-principles.md`:

1. One primary action per screen.
2. Type carries hierarchy.
3. Snap to the 4/8 grid.
4. Concentric corners.
5. Sub-100 ms or it's broken.
6. Skeletons not spinners.
7. Keyboard is a first-class surface.
8. `prefers-reduced-motion` honoured everywhere.
9. One icon family, one stroke weight.
10. Copy is warm, direct, present-tense, sentence-case.

Any PR that breaks one of these has to call it out and explain.

### Storage keys are opaque

- Storage keys are `ulid::Ulid::new()` (or UUIDv7). Never derived from user input.
- Display names are separate metadata, sanitised on store and re-sanitised on render.
- `fs` adapter canonicalises and root-confines every resolved path. Refuses symlinks escaping the root.

### Security checklist applies to every PR

The v0 must-have list at the bottom of `docs/research/06-security.md` is the gate. New endpoints get reviewed against:
- magic-byte sniffing on uploads
- `nosniff` + per-origin CSP
- rate limit + body cap + quota
- redaction of `Authorization`, `Cookie`, `X-WOPI-*`, `?access_token=` in logs
- HMAC constant-time compares for any signed token

## Working rules for Claude

1. **Read before you write.** When implementing a feature, read the relevant flow + surface spec + architecture section first. Cite paths in PRs.
2. **Match the existing tone.** This repo's docs (and sibling sheet/, document/) use terse, decision-oriented language. No marketing prose. No exclamation marks.
3. **Cite file paths and line numbers** when referencing existing code: `crates/drive-wopi/src/handlers.rs:142`.
4. **Default new editor-side code to the sibling repos.** Sheet WOPI client logic goes in `../sheet/apps/server/` and `../sheet/apps/web/`. Document WOPI client logic goes in `../document/backend/internal/host/wopi/` and `../document/docx-editor/`.
5. **Don't propose unbacked alternatives.** WOPI is the editor handoff. OpenDAL is the storage layer. tower-sessions is the session layer. Argon2id is the password hash. These are locked. Reopening requires new research and a new synthesis update.
6. **Update docs in the same commit as the code change.** If you change a flow, update `01-flows.md`. If you change the storage facade, update `ARCHITECTURE.md` §"Storage facade". Stale docs poison every future session.
7. **Don't introduce new runtime dependencies casually.** Adding a crate widens the bus factor and the vuln surface. Justify in the PR.
8. **Test against the conformance suite.** Storage changes run all four adapter tests. WOPI changes run the proof-key fixtures (even though we don't validate in v0 — we will).

## Style

- Match the tight tone of `../sheet/CLAUDE.md` and `../document/CLAUDE.md` when adding to docs.
- Don't bloat docs with marketing language.
- State decisions and tradeoffs.
- Use the citation format `crates/drive-X/src/file.rs:LINE` when referencing source.

## Phase awareness

Always know which phase we're in. The current phase is at the top of `PLAN.md`. As of writing:

- **Phase 0 — spikes** is next. Throwaway code to prove unknowns.
- Phase 1 walking-skeleton scope is locked but not started.
- Phase 2 + 3 are described but not committed.

Don't start Phase 1 code until Phase 0 spikes are decided.
