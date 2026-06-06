# PLAN

Phased delivery plan for Casual Drive. Each phase has scope, non-goals, acceptance tests, and dependencies. Read this with [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md), [`docs/ux/01-flows.md`](./docs/ux/01-flows.md), and [`docs/ux/02-surface.md`](./docs/ux/02-surface.md) next to it.

Posture: never break the five inviolable rules — research first, plan UX, consistent UI, industry-standard secure code, macOS-grade polish. The research and the UX/surface specs are now done; from here it's implementation against them.

---

## Phase 0 — Spikes (riskiest unknowns first)

**Goal.** Prove the unknowns are solved before committing to Phase 1's architecture. Throwaway code allowed.

**Scope.**

1. **Storage facade spike** — minimal `Storage` struct wrapping `opendal::Operator`, with `put`/`get`/`stat`/`signed_get`. Conformance test runs against fs + memory + MinIO-in-testcontainers. ~1 day.
2. **WOPI host spike** — Drive serves CheckFileInfo + GetFile + PutFile + Lock + Unlock + RefreshLock against in-memory state. No discovery doc yet. Verified with a hand-crafted client script. ~2 days.
3. **WOPI client spike in sheet/** — add `/hosting/discovery` + `/wopi/editor` entry route to sheet/'s Fastify server. Open a Drive-hosted `.xlsx` end-to-end (Drive's spike host ↔ sheet's spike client). ~1–2 days, mostly in sheet/.
4. **Two-origin spike** — Axum binary serving two `Host`-differentiated routers, boot-time origin-mismatch refusal, the `/raw/{token}` HMAC handler. ~0.5 day.
5. **SPA shell spike** — Vite + React + Tailwind + tokens from [`research/04-polish-principles.md`](./research/04-polish-principles.md) loaded; the empty-state surface from [`docs/ux/02-surface.md`](./docs/ux/02-surface.md) §7 renders in light + dark mode at the right density. No data wiring yet. ~1 day.

**Non-goals.** No persistence beyond what each spike needs. No auth. No production deploy. No cross-repo PRs to sheet/ — spikes branch locally, get scrubbed.

**Acceptance.**

- [ ] Storage conformance suite green against fs + memory + MinIO.
- [ ] WOPI spike survives a 10-min co-edit simulation: lock acquired, refreshed twice, PutFile lands, unlock on disconnect.
- [ ] End-to-end open of a Drive-hosted `.xlsx` in sheet shows the workbook and a save round-trips.
- [ ] Two-origin spike: requests to the wrong origin return 421. Boot in prod with matching origins refuses.
- [ ] SPA empty-state pixel-matches the §7 spec in both themes.

**Exit gate.** Each spike's outcome is summarised in `docs/spikes/0X.md` with what worked, what surprised, what to revise in the Phase 1 design.

---

## Phase 1 — Walking skeleton

**Goal.** A single-tenant Drive an admin can deploy, sign into, upload to, browse, open `.xlsx` in sheet/ and `.docx` in document/, download from. Polished enough to dogfood.

**Scope.**

### Storage + data
- `Storage` facade hardened: full `Storage` trait API, error mapping, capability gates, conformance suite (fs / memory / MinIO; S3 opt-in).
- SQLite schema (with portable migrations also valid against Postgres): `users`, `files`, `folders`, `share_links`, `wopi_locks`, `sessions`.
- `sqlx` migration runner on boot.
- Soft-delete: `trashed_at` column on `files`/`folders`. 30-day janitor job (deferred to Phase 2 — for now Trash is forever).

### Auth (single-tenant)
- `/sign-in` page + handler. `__Host-cd_sid` cookie, Argon2id (`m=19MiB, t=2, p=1`).
- Rate-limiter on `/sign-in` (10/min/IP, exponential backoff per-account after 5 fails).
- `/sign-out`. Session expiry → 401 flow per [`flows §3`](./docs/ux/01-flows.md).
- CSRF: `__Host-` cookie + same-site Lax + token on state-changing routes.

### File API
- `GET /api/folders/<id>` — listing (paginated).
- `POST /api/files` — multipart upload, streaming to `Storage::put_stream`. Body cap 100 MB (env-configurable). Magic-byte sniffing (`infer`), display-name sanitisation, opaque storage keys (ULID).
- `POST /api/folders` — create.
- `PATCH /api/files/<id>` / `/api/folders/<id>` — rename only (move uses bulk route).
- `POST /api/files/move` — bulk move with conflict handling.
- `POST /api/files/trash` — bulk soft-delete; locks-respected (open files refused).
- `POST /api/files/restore` — undo trash.
- `GET /api/files/<id>/download` — 302 to signed URL.

### WOPI host
- All 7 endpoints from [`ARCHITECTURE.md §"Endpoint contract"`](./docs/ARCHITECTURE.md). `wopi_locks` table backing.
- `GET /api/files/<id>/open` mints a launch URL + access token.
- Per-call access-token validation; perms enforcement (read blocks PutFile, etc.).

### WOPI clients (cross-repo PRs)
- **sheet/**: `/hosting/discovery` route + `/wopi/editor` iframe entry + a lock-refresh loop + `beforeunload` Unlock-via-beacon.
- **document/**: implement the `wopi` `host.Integration` impl in `backend/internal/host/wopi/` against this Drive binary.

### Two-origin model
- `app_origin` vs `usercontent_origin` split enforced by `host_dispatch_layer`.
- `/raw/{token}` mounted on user-content origin only.
- CSP per origin per [`ARCHITECTURE.md §"Two-origin security model"`](./docs/ARCHITECTURE.md).
- Headers: HSTS, Referrer-Policy, Permissions-Policy, `nosniff`, `Content-Disposition: attachment` on user-content.

### SPA
Flows 1–11 and 13 from [`01-flows.md`](./docs/ux/01-flows.md):
1. First-run empty state
2. Sign in
3. Sign out + session-expiry handling
4. Upload (button + drag-drop + folder)
5. Browse
6. Open file in editor (WOPI handoff)
7. Rename
8. Create folder
9. Move (drag + picker)
10. Delete → trash
11. Restore from trash (basic; Empty Trash deferred to Phase 2)
13. Search (Cmd-K palette) — files only; commands list from flow 13's spec but limited to the actions implemented above

Surfaces from [`02-surface.md`](./docs/ux/02-surface.md): app shell, sidebar, top bar, breadcrumbs + sort header, file list, empty states, command palette, four modals (Empty Trash deferred to Phase 2; others ship), toasts, drop zones, inline upload row, sign-in card. Editor badge.

### Build & deploy
- `cargo-chef` Dockerfile.
- `docker-compose.dev.yml` with MinIO sidecar.
- Caddyfile example for prod.
- `.env.example` complete.

### Tests
- Storage conformance suite (carry-over from Phase 0, expanded).
- WOPI handler tests with proof-key fixtures (validation hook present, returns "skipped" verdict in v0).
- SPA E2E in Playwright: full happy-path of every Phase-1 flow.
- Cross-repo WOPI E2E (in sheet/, extended `playwright.wopi.config.ts` pointing at this Drive).

**Non-goals.**

- Share-links (Phase 2).
- Empty-trash + 30-day janitor (Phase 2).
- Multi-select bulk actions beyond what bulk endpoints already need (Phase 2).
- Selection bar UI (Phase 2; rows still cmd-clickable, just no bottom bar).
- Settings page (Phase 2).
- Preview pane (Phase 2).
- Cmd-K commands beyond the bare minimum (Phase 2).
- OIDC / multi-user (Phase 3+).
- Casual Slides handoff (Phase 3+).
- Proof-key cryptographic validation (Phase 3+ if MS365 federation is ever enabled).
- Auto-thumbnailing (Phase 3+, requires sandboxed image worker).
- Mobile layout polish (Phase 3+).

**Acceptance.**

- [ ] Admin can deploy via `docker compose up`, sign in, upload a 50 MB `.xlsx`, open it in sheet/, edit it, save, return to Drive, see the updated `Modified` time.
- [ ] Same for `.docx` in document/.
- [ ] 100-file upload completes; ghost rows turn real; one batch toast at the end.
- [ ] Rename works inline, optimistic, with collision error inline.
- [ ] Move (drag + picker) works to nested folders.
- [ ] Trash + restore works.
- [ ] Cmd-K opens, finds files, runs the few implemented commands.
- [ ] All five inviolable rules pass an honest self-check at PR review.
- [ ] Cross-origin XSS attempt (upload an HTML file, try to navigate to it from app-origin) fails — file served from user-content origin with `attachment` + sandbox CSP.
- [ ] OWASP must-have checklist from [`research/06-security.md`](./docs/research/06-security.md) all green.
- [ ] First-paint < 1 s on the reference $5-VPS deploy.
- [ ] Sub-100 ms feedback on every direct manipulation.
- [ ] `prefers-reduced-motion` honoured in all motion paths.
- [ ] Light + dark themes both ship.

**Estimated wall-clock.** 4–6 weeks of focused work for Drive + Sheet/Document retrofit, assuming Phase 0 spikes go cleanly.

---

## Phase 2 — Share, polish, parity with Linear-tier expectations

**Goal.** Drive feels finished. The selection-bar workflow lands. Share-links ship. Cmd-K becomes the safety net it's specced as.

**Scope.**

### Share-links
- Flows 14–15 from [`01-flows.md`](./docs/ux/01-flows.md): create / manage / recipient open.
- Share modal (flow 14).
- Recipient share page (surface 14).
- 128-bit random tokens; optional Argon2id-hashed password; expiry default 7d; permissions enum (view / view+download / edit).
- Share-link consumer flow into the WOPI launcher (Phase 1 already mints WOPI tokens from sessions; this adds share-link → WOPI token minting).

### Multi-select + bulk actions
- Selection bar surface (§8) ships.
- Flows 12 + bulk-action conflict resolver (modal §10.4).

### Trash polish
- Empty Trash modal (§10.1).
- 30-day janitor (Postgres `pg_cron` / SQLite background job depending on backend).
- Restored-to-Home fallback when original parent doesn't exist.

### Polish pass against the 10 commandments
- Audit every surface against the checklist at the bottom of [`02-surface.md`](./docs/ux/02-surface.md) ("States checklist (per surface)").
- Skeleton loading states on file list and Cmd-K results.
- Pre-fetch on folder hover.
- Optimistic UI on rename and move (already in Phase 1) extended to: star/unstar, trash (with undo within toast lifetime), restore.
- FLIP animations on list reflow.
- Tabular numerals everywhere.
- Caps-lock detection on sign-in.

### Cmd-K commands
- Full command list from flow 13: New folder, Upload, Toggle theme, Sign out, Empty Trash, plus any other actions we've shipped.

### Settings page
- Change password, theme override, antivirus toggle (no-op for now), recipient-footer toggle, body-limit display, current backend display.

### Cross-cutting

- Antivirus adapter trait shipped (no-op default); ClamAV adapter behind cargo feature.
- Audit log (append-only table — pre-hash-chain; full hash-chained version is Phase 3).
- More E2E coverage; flaky-test budget = 0.

**Non-goals.**
- OIDC / multi-user.
- Preview pane.
- Auto-thumbnailing.
- Casual Slides handoff.
- Public landing page (single-tenant install — direct unauthenticated visits already redirect to sign-in).
- Mobile polish.
- File properties dialog (Phase 3).

**Acceptance.**

- [ ] Create share-link, send URL to a fresh browser, open file (view/edit per perms).
- [ ] Password-protected link gates with the right page.
- [ ] Revoke link → 404 within 1 s for any concurrent viewer.
- [ ] Multi-select 20 items → bulk move → 1 toast with undo → undo works.
- [ ] Empty Trash modal requires explicit confirm; janitor evicts 30+-day-old items.
- [ ] Pre-fetch on hover reduces visible loading state on subsequent navigation.
- [ ] All "States checklist" boxes ticked.

**Estimated wall-clock.** 3–4 weeks.

---

## Phase 3 — Optional surfaces (pick by need)

These are independently picked; none gate the others. Pick as the dogfood reveals priorities.

### 3a — Preview pane
Inline preview for images, PDFs, plain text, markdown. Quick-Look-style spacebar invocation. New surface in [`02-surface.md`](./docs/ux/02-surface.md); flows extension.

### 3b — File properties dialog
Full metadata, sniffed type, hash, sharing history, editor sessions.

### 3c — Multi-user accounts
OIDC via `openidconnect`. Per-user file ownership. The `AuthnBackend` trait Phase 1 already exposes is the slot. Schema migration to add `user_id` FKs across `files`/`share_links`. Admin UI to manage users.

### 3d — Mobile polish
The narrow-viewport hooks noted in surface specs become real (vaul drawer sidebar, stacked card list view). Touch-friendly drag-drop alternatives.

### 3e — Casual Slides handoff
Wire `.pptx` to the (now-extant) Casual Slides editor. Mirror the sheet/document retrofit pattern.

### 3f — MS365 federation
Proof-key RSA validation. Discovery doc proof-key refresh on validation failure. WOPI Validator harness in CI. New `frame-ancestors` allowlist entries.

### 3g — Auto-thumbnailing
Sandboxed image worker (libvips in a separate process or wasm). Phase 2's antivirus quarantine pattern transfers.

### 3h — Audit log hash-chain
Append-only chain with periodic anchor publication. Useful when multi-user lands.

### 3i — Tauri desktop wrapper
Match the suite's "Casual Desktop" lane referenced in `../site/`. Drive shell in a native window.

---

## What stays out forever (unless reframed)

- **Multi-document editing in one window.** Drive launches editors; it doesn't reimplement them.
- **Native macOS app.** Tauri wraps the web shell; we don't ship a separate AppKit/Catalyst app.
- **Sync clients (Dropbox-like).** Self-host pitch is "browser is enough." Sync is a separate product category.
- **Real-time presence in the file list.** "Who else is viewing this folder" — out of scope. Real-time presence happens inside editors via their own collab layer.

---

## Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| WOPI retrofit in sheet/ takes longer than estimated due to Fastify quirks | Med | Phase 0 spike #3 catches this early |
| OpenDAL pre-1.0 churn breaks Drive across patch bumps | Med | Pin exact version; audit `docs/upgrade` on every bump; conformance suite catches regressions |
| Two-origin operational burden (cert, DNS) annoys self-hosters | Low–Med | Ship Caddy config that does both with one config block; document `nip.io`-style dev pattern |
| Cross-repo coordination with sheet/ + document/ creates merge churn | Med | Spike #3 surfaces the wire format; once frozen, the two sides move independently |
| Antivirus integration drags Phase 2 | Low | Adapter trait + no-op default already separates "ship the hook" from "ship the implementation" |
| Argon2id memory cost makes login slow on $5 VPS | Low | Profiled; 19 MiB × 2 iterations is ~40 ms on a 1 vCPU box. Fine. |
| Browser blocks the `window.open(...)` in flow 6 because of strict popup blockers | Med | Direct user-gesture invocation only; document the rare failure case in copy ("Couldn't open — please allow popups for Casual Drive") |

---

## When are we done planning?

Now. The research, UX flows, surface spec, architecture, and milestone plan together are the planning artefact set. Implementation starts at Phase 0 spike #1 the moment we choose to go.
