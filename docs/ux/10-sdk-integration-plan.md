# 10 — SDK integration plan (Casual Editor / Sheets in-Drive)

**Revised 2026-06-08** — SDK is the primary integration. Iframe path is out of scope for Phase 1; the existing WOPI new-tab handoff covers the isolated-launch case.

Companion to `08-editor-handoff.md` (current WOPI new-tab handoff) and `07-preview-surface.md` (where the editor mounts). Source contracts: [`13-iframe-protocol.md`](https://github.com/schnsrw/docx/blob/main/docs/internal/13-iframe-protocol.md) + [`14-sdk-delivery.md`](https://github.com/schnsrw/docx/blob/main/docs/internal/14-sdk-delivery.md).

## What's already shipped

- ✅ **WOPI handoff** (`08-editor-handoff.md`, pipeline row 4.3 + 4.4). `GET /api/files/{id}/open` mints a per-launch token + redirects to the editor in a new tab. The editor calls back to Drive's `/wopi/files/{id}` endpoints for CheckFileInfo / GetFile / PutFile + Lock lifecycle. **Continues to work as the third-party WOPI path** — don't touch it.
- ✅ **`crates/drive-wopi`** — WOPI host implementation (7 endpoints, 409 lock contract, token mint via `mint_token`).
- ✅ **`@schnsrw/docx-js-editor@1.0.0`** — `CasualEditor` React component + `FileSource` interface, on npm.
- ✅ **`@schnsrw/casual-sheets@0.3.0`** — `CasualSheets` React Univer wrapper, signing surface, iframe protocol types, on npm. Eager Univer plugin CSS at `@schnsrw/casual-sheets/styles`.

## What this plan proposes

**Phase 1 — SDK integration (in-Drive editing).** Drive `npm install`s the editor SDKs and mounts them directly into its React tree. The Preview modal's stage becomes a real editor for `.docx` / `.xlsx` instead of a procedural-thumbnail placeholder. Bytes flow through Drive's own content endpoints (no WOPI dance for the SDK path); the WOPI handoff stays around for the third-party launch.

**Why SDK specifically:**

- **One container by default.** Drive bundles the editor; no second deploy for the editor's gateway. Operators flip `DRIVE_COLLAB_BACKEND_URL=wss://...` to enable real-time co-edit (which then needs the Casual gateway as a second container) — but that's opt-in, not the default.
- **No postMessage hop.** The editor shares Drive's React state directly; drag-from-sidebar, user-identity propagation, etc. compose naturally.
- **Same security model as the rest of Drive.** Bytes ride the existing same-origin authenticated session — no separate CSP `frame-ancestors` story, no token mint per launch.

### Out of Phase 1 scope

- **Iframe-in-Preview** — the editor's `/embed` route is already shipped, and Drive could host it via postMessage. We're skipping it because the SDK gives us in-React mounting that's strictly better for the single-tenant, single-container case Drive optimises for. Iframe revisits only if a real need surfaces (multi-tenant isolation, cross-org embedding, etc.).
- **Signature pipeline** — Phase 2 (was Phase C). Drive becomes a signing host: opens the editor in signing mode, persists per-field bytes + audit rows. Editor surface is already shipped at `@schnsrw/casual-sheets/signing` + `@schnsrw/docx-js-editor`; backend stamping lands in its own PR.

---

## Phases (numbered)

### Phase 1 — SDK + DriveFileSource (in-Drive editing)

Drive imports `@schnsrw/docx-js-editor` + `@schnsrw/casual-sheets`, mounts them under the Preview modal's stage, and routes bytes through new content endpoints.

1. **P1.1 — Drive backend.** Two new endpoints alongside the existing WOPI surface — same-origin, session-cookie + CSRF auth, no token mint:
   - `GET /api/files/{id}/content` — returns the raw bytes inline (200 with body). Lets the SDK consume bytes without the WOPI `wopiSrc` + token round trip.
   - `PUT /api/files/{id}/content` — accepts raw bytes in the body, writes through `crates/drive-storage`, updates `size` + `updated_at` on the file row, emits a `files.save` audit event.

   ~80 Rust lines in `crates/drive-http/src/files.rs`, alongside the existing handlers.

2. **P1.2 — `DriveFileSource` (~80 TS lines).** Implements the editor's `FileSource` interface against the new content endpoints. Methods:
   - `open(docId)` → `GET /api/files/{id}/content` → `{ name, contents: Uint8Array }`
   - `save(docId, bytes)` → `PUT /api/files/{id}/content`
   - `list / rename / delete / watchRecent / rememberLastOpened / lastOpened` → no-op (Drive owns those UIs in its own chrome; the editor never invokes them when these no-op).

   Lives at `web/src/file-source/DriveFileSource.ts`.

3. **P1.3 — React wrappers.** Two thin components that wrap the SDK with `DriveFileSource` + Drive's user identity:
   - `web/src/components/editor/CasualDocEditor.tsx` (~80 TS lines) wraps `<CasualEditor>` from `@schnsrw/docx-js-editor`.
   - `web/src/components/editor/CasualSheetWorkspace.tsx` (~80 TS lines) wraps `<CasualSheets>` from `@schnsrw/casual-sheets/sheets`. Imports `@schnsrw/casual-sheets/styles` once.

4. **P1.4 — Preview wiring.** `web/src/components/preview/PreviewStage.tsx` gets two new cases:
   - `kind === 'doc'` → `<CasualDocEditor fileId={file.id} />`
   - `kind === 'sheet'` → `<CasualSheetWorkspace fileId={file.id} />`

   Removes the procedural-thumbnail placeholder for those kinds.

5. **P1.5 — Co-edit env flag.** Operator env `DRIVE_COLLAB_BACKEND_URL=wss://collab.drive.example` propagates to the SPA via the existing `/api/about` / config endpoint. When set, the wrappers pass it to `<CasualEditor backendUrl=...>` / `<CasualSheets>` (collab plumbing in the sheet SDK lands when its wrapper supports it). When unset, Drive runs as one container.

**Where things live:**

- `crates/drive-http/src/files.rs` — adds `get_content` + `put_content` handlers + two routes. ~80 Rust lines.
- `web/src/file-source/DriveFileSource.ts` — new file. ~80 TS lines.
- `web/src/components/editor/CasualDocEditor.tsx` — new file. ~80 TS lines.
- `web/src/components/editor/CasualSheetWorkspace.tsx` — new file. ~80 TS lines.
- `web/src/components/preview/PreviewStage.tsx` — extended with 2 cases.
- `web/package.json` — adds `@schnsrw/docx-js-editor` + `@schnsrw/casual-sheets` (peer Univer set: 15 `@univerjs/*` packages at `^0.24.0`).

**Out of scope:** signature pipeline (Phase 2).

### Phase 2 — Signature pipeline (was Phase C)

Drive becomes a real signing workflow host: user clicks "Sign this file" → Drive opens the editor in signing mode → user signs anchored fields → Drive's Rust backend stamps the signatures + writes an audit row.

1. **C1 — Audit table.** Migration adds `signature_sessions` + `signature_fields` rows. Schema:

   ```sql
   CREATE TABLE signature_sessions (
     id          TEXT PRIMARY KEY,           -- ULID
     file_id     TEXT NOT NULL REFERENCES files(id),
     started_by  TEXT NOT NULL,              -- user id
     started_at  INTEGER NOT NULL,           -- unix seconds
     mode        TEXT NOT NULL,              -- 'sequential' | 'concurrent'
     completed_at INTEGER,                   -- null until session done
     cancelled_at INTEGER,
     cancel_reason TEXT
   );
   CREATE TABLE signature_fields (
     id            TEXT PRIMARY KEY,         -- ULID
     session_id    TEXT NOT NULL REFERENCES signature_sessions(id),
     field_id      TEXT NOT NULL,            -- client-side field id
     label         TEXT NOT NULL,
     required      INTEGER NOT NULL,         -- 0/1
     anchor_kind   TEXT NOT NULL,            -- 'doc' | 'sheet'
     anchor_para_id TEXT,                    -- for docs
     anchor_sheet  TEXT,                     -- for sheets
     anchor_cell   TEXT,                     -- for sheets
     method        TEXT,                     -- null until signed
     signature_bytes_path TEXT,              -- blob storage path
     signed_at     INTEGER,
     signer_user_id TEXT
   );
   ```

   Portable across SQLite + Postgres per CLAUDE.md hard rule.

2. **C2 — Backend endpoints.**
   - `POST /api/files/{id}/sign` — opens a signing session. Body: the `SignatureField[]` array + mode. Response: `{session_id, signing_url}` where `signing_url` is the editor's `/embed?app=docs&signing=<base64-session-id>&...`.
   - `POST /api/sign-sessions/{session_id}/fields` — editor posts per-field bytes via the postMessage bridge → Drive's SPA forwards to this endpoint → Rust persists the bytes to `crates/drive-storage` + writes the audit row.
   - `POST /api/sign-sessions/{session_id}/complete` — fires when all required fields are done; Drive's Rust side stamps the bytes into the workbook/doc using `ring` (or `umya-spreadsheet` for sheet) + writes the final etag back via the existing WOPI PutFile path.
   - `POST /api/sign-sessions/{session_id}/cancel` — fires on `casual.signature.cancel`. Writes the cancel reason + leaves the file untouched.

3. **C3 — Drive SPA.** "Sign this file" action in the Preview modal opens the editor iframe with `signing` config. The postMessage bridge listens for `casual.signature.field.signed` / `casual.signature.complete` / `casual.signature.cancel` and forwards to the corresponding `/api/sign-sessions/...` endpoint.

4. **C4 — Identity attestation.** The session's `signer_user_id` comes from Drive's authenticated session (`__Host-cd_sid`). The editor doesn't choose who the signer is; Drive does. The editor receives the signer's name + email in the `SignatureField.signer` field purely as a UX hint (rendered next to "Type your name").

**Where things live:**

- `crates/drive-db/migrations/` — new migration. SQLite + Postgres compatible.
- `crates/drive-http/src/handlers/signing.rs` — new file. ~250 Rust lines.
- `crates/drive-storage` — extends with a `put_signature_blob(session_id, field_id, bytes) → path` method.
- `crates/drive-signing/` — **new crate**. Owns the stamping logic + the `ring` dep so the rest of Drive stays crypto-free. ~300 Rust lines.
- `web/src/components/preview/EditorIframe.tsx` — extended with the signature postMessage handlers.
- `web/src/components/signing/SigningButton.tsx` — new "Sign this file" affordance. ~100 TS lines.

**Out of scope:**

- **PKI-grade signing (X.509 detached signatures).** Drive's v0 signing pipeline produces drawn / typed / uploaded images stamped into the bytes. CA-issued signatures are a v0.2 extension on the existing `signature_bytes_path` blob — the protocol carries opaque bytes; the crypto choice is the host's.
- **Multi-party sequential delegation across signers** ("Alice signs → email Bob → Bob signs"). The protocol supports `sequential` mode within a single session; cross-session orchestration is a Drive feature, not editor protocol.
- **Field placement UI inside Drive.** Phase C ships with operator-supplied field arrays only. Drive's "click to place a signature here" UI is Phase D.

---

## Sequence — Phase 1 (SDK + DriveFileSource)

```
Drive SPA                              Drive backend
─────────                              ─────────────
user clicks .docx row in Preview
      │
      ▼
construct DriveFileSource(fileId)
mount <CasualDocEditor fileId={fileId} />
       │ wraps <CasualEditor fileSource={fs} docId={fileId} />
       ▼
fs.open(fileId)
       │
       ▼
GET /api/files/{id}/content (same-origin cookie + CSRF)
                                     ──► storage.get(storage_key(id))
                                     ◄── bytes
       ◄── 200 application/octet-stream
       │
render workbook / document
…
user types / formats
…
useFileSourceAutoSave → fs.save(fileId, bytes)
       │
       ▼
PUT /api/files/{id}/content
       │
                                     ──► storage.put(storage_key(id), bytes)
                                          file_repo.update_size_and_mtime
                                          audit.emit("files.save")
                                     ◄── 200 FileDto
       ◄── 200
Drive's recent-files strip refreshes from FileDto.updated_at
```

Same-origin, same auth cookie, no token mint. Iframe + postMessage stay shipped at the editor side for future use; Phase 1 doesn't touch them.

---

## Open questions for sign-off

1. **Phase ordering?** ✅ confirmed 2026-06-08 — Phase 1 SDK first, Phase 2 signing later, iframe out of scope for now.
2. **Co-edit default in Phase 1?** ✅ confirmed off by default — operators flip `DRIVE_COLLAB_BACKEND_URL` to enable.
3. **Signature blob storage backend?** Default: `crates/drive-storage`'s existing OpenDAL facade (fs / s3 / memory / minio). Alternative: dedicated signatures bucket. **Decide at Phase 2.**
4. **Audit row partitioning?** Default: one row per session + one per field. **Decide at Phase 2.**
5. **Signing crate naming?** Default: `crates/drive-signing`. Keeps the crypto dep (`ring`) isolated. **Decide at Phase 2.**
6. **Field anchor UI?** Default: operator-supplied field arrays via API (no in-Drive placement UI in Phase 2). **Decide at Phase 2.**

---

## What this plan does NOT change

- The existing WOPI new-tab handoff (`08-editor-handoff.md`). Both paths coexist.
- The two-origin model (`drive.<host>` for app, `usercontent-drive.<host>` for raw bytes). The SDK content endpoints serve from the app origin (small bytes inline); the existing user-content origin keeps the download flow.
- The single-tenant admin auth model. SDK uses the existing `__Host-cd_sid` session + CSRF.
- The "no multi-user accounts in v0" rule. Phase 2 signatures are signed by the authenticated admin; multi-user signing is a v0.2+ feature.

## Required reading before code lands

1. This doc.
2. [Casual Editor SDK delivery](https://github.com/schnsrw/docx/blob/main/docs/internal/14-sdk-delivery.md) — `FileSource` interface contract.
3. [Casual Sheets signing + embed](https://github.com/schnsrw/sheets/blob/main/docs/SDK_SIGNING_EMBED.md).
4. `08-editor-handoff.md` (existing WOPI path).
5. `crates/drive-http/src/files.rs` — current file routes (add content endpoints alongside).

## Estimated effort (rough)

| Phase | Rust | TS | Tests | Notes |
| ----- | ---- | -- | ----- | ----- |
| 1 | ~80 LOC | ~240 LOC | ~150 LOC | Both SDKs live; ships in one PR |
| 2 | ~550 LOC (new crate) | ~250 LOC | ~250 LOC | New crate + migrations; needs Q3/4/5/6 decisions |

Numbers are pre-code estimates; expect ±30%.

---

## Why this plan exists

Drive's "plan → present → ask → code" rule (CLAUDE.md §"Default working mode") means substantive features land plan-first. This is the plan; the implementation lands in subsequent PRs phase-by-phase, each with its own narrower review.
