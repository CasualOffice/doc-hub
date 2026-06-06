# 01 — Core UX Flows

The interaction-level spec for Casual Drive v0. Sixteen flows. No pixel mockups — that's [`02-surface.md`](./02-surface.md) (next). This doc answers *what happens*, *in what order*, *what does the user see and feel*, *what's the keyboard*, *what's the copy*.

Calibration: Drive's polish bar is the macOS-app reference set in [`../research/04-polish-principles.md`](../research/04-polish-principles.md) — Things 3, Linear, Raycast, Notion, Sonoma system apps. The 10 commandments at the bottom of that doc are a checklist every flow below must pass.

Convention per flow:

- **Goal** — one sentence; the user's intent.
- **Trigger** — how the user gets here.
- **Happy path** — numbered steps describing what the system does and what the user sees.
- **Keyboard** — shortcuts active in this flow.
- **Copy** — exact strings (button labels, errors, toasts).
- **Polish notes** — motion, focus, timing, microcopy — the details that turn it from "works" into "feels Drive-y".
- **Edge cases** — failure modes and recovery.
- **Success criteria** — what counts as the flow shipped.

Cross-cutting invariants every flow honours:

- Sub-100 ms feedback on every direct manipulation (commandment #5).
- Skeletons not spinners for content (#6).
- Every important action has a shortcut, advertised next to it (#7).
- `prefers-reduced-motion` respected (#8).
- Optimistic UI for any plausibly-safe write; reconcile + roll back on failure.
- One toast for the whole batch on bulk actions — never N toasts for N items.

---

## 1 — First-run empty state

**Goal.** A brand-new admin opens Drive at `https://drive.<host>/` for the first time and understands what to do.

**Trigger.** Fresh deploy, no files yet, admin signed in.

**Happy path.**

1. App shell renders instantly (cached + embedded). Sidebar collapsed to its 240 px default; main pane shows the empty state.
2. Empty state: centred 56 × 56 Lucide `folder-open` glyph in `--fg-subtle`, then on a new line in `--fg-default text-xl semibold` the title **"Your Drive is empty."**, then a single muted line in `--fg-muted text-md` **"Drop files anywhere, or use Upload."**, then *one* primary button **"Upload"**.
3. The whole window is a drop target. Hovering files over the window darkens the canvas to `--bg-subtle` and shows a centered card: dashed 2 px `--accent-muted` border, glyph `upload-cloud`, copy **"Drop to upload to *Home*"** (folder name reflects the current location).

**Keyboard.** `U` opens the file picker. `Cmd-K` opens the palette. Both shortcuts are visible — the Upload button shows `U` in muted text on the right.

**Copy.**
- Title: **"Your Drive is empty."**
- Subtitle: **"Drop files anywhere, or use Upload."**
- Primary button: **"Upload"** + chord chip `U`.

**Polish notes.**
- Empty state fades in at 200 ms after shell mount; no flash of nothing.
- Drop-zone activation transition: `120 ms --ease-out` background fill + border colour. Pop-in card uses spring `{stiffness: 400, damping: 30}`.
- Lucide glyph never spins or pulses — restraint.

**Edge cases.**
- Drop fails (permission/size/scan): rollback the drop animation, show toast (see flow 4).
- User drops a folder when on a backend that doesn't support directories: still works — folder structure flattens into prefix-joined keys client-side (OpenDAL handles it).

**Success criteria.**
- Time-to-first-meaningful-paint < 1 s on the $5-VPS reference deploy.
- User can complete an upload from this screen without reading documentation.
- No tutorial overlay. No "first launch" modal.

---

## 2 — Sign in (single admin)

**Goal.** The single admin authenticates with their env-seeded password and lands on Drive's root.

**Trigger.** Unauthenticated request to any `/files/*` route → redirect to `/sign-in`.

**Happy path.**

1. `/sign-in` renders a centred card on the canvas: Lucide `cloud` glyph, "Casual Drive", a single password input, a primary **"Sign in"** button. No email/username field — single-tenant.
2. Focus auto-lands on the password input. The card is `--radius-lg`, `--shadow-md`, hairline border. Background canvas can be the marketing accent gradient or just `--bg-canvas` — restrained.
3. User types, hits `Enter` or clicks **Sign in**.
4. Button shows a thin inline progress bar across its bottom edge (not a spinner inside the label). The label stays the same; the button doesn't move.
5. On 200: redirect to the original `/files/*` or to `/` if none. Session cookie set: `__Host-cd_sid=...; Path=/; Secure; HttpOnly; SameSite=Lax`.
6. On 401: input gets a 1 px `--danger` border and one-line helper text below: **"Wrong password."** Card shakes once horizontally (8 px, 250 ms, eased) — Apple-style, not a long jiggle.

**Keyboard.** `Enter` submits. `Tab`/`Shift-Tab` cycle the few focusable items.

**Copy.**
- Heading: **"Casual Drive"**.
- Subheading (muted, optional): **"Sign in to continue."**
- Placeholder: **"Password"**.
- Button: **"Sign in"**.
- Error: **"Wrong password."** (never "Invalid credentials" — there's only the admin; we don't pretend otherwise).
- Account-locked variant after 5 failures: **"Too many attempts. Try again in 10 minutes."**

**Polish notes.**
- No "Remember me" checkbox — server decides cookie lifetime.
- No social login icons (no OAuth in v0).
- No marketing footer — this is an admin page, not a SaaS landing.
- Errors **never** disclose whether the password was wrong or the account is locked, except for the rate-limit message which is intentional UX.
- Caps-lock detection: if pressed while typing, show a subtle muted line **"Caps Lock is on."** Apple convention.

**Edge cases.**
- Rate-limit hit (10/min/IP, 5/account): button stays disabled with the lockout message; countdown is **not** shown live (Apple doesn't either — just the static "10 minutes").
- Browser without cookies enabled: render a banner **"Cookies are required."** above the card.
- HTTPS not enforced in dev: still set `Secure` cookie; in dev compose, the dev origin is also https (Caddy local cert).

**Success criteria.**
- Sign-in succeeds with the env-seeded password.
- All CSRF/cookie attributes verified in an integration test.
- Lockout test passes (6th attempt within a minute is blocked with the right message).

---

## 3 — Sign out + session expiry

**Goal.** User signs out cleanly, or their session expires gracefully without losing in-progress work.

**Trigger.** Sign-out: user clicks **Sign out** in the avatar menu (top-right of sidebar) or hits its shortcut. Expiry: any backend call returns 401 due to cookie expiry.

**Happy path — sign out.**

1. User clicks avatar (top of sidebar, single-letter monogram on `--bg-subtle`) → menu opens (Radix dropdown, slide+fade 150 ms): **Account**, **Settings**, separator, **Sign out** (with `Shift-Cmd-Q` chord on the right, muted).
2. On click, modal-less confirm: just immediate POST to `/sign-out`. The shell fades to half-opacity for 80 ms, then redirects to `/sign-in`.
3. Server clears `__Host-cd_sid` (`Set-Cookie: ...; Max-Age=0`).

**Happy path — session expiry.**

1. Any backend call returns 401. The store catches it.
2. Toast (sonner, top-right): **"Signed out for security."** with action **"Sign back in"**.
3. App stays mounted in read-only state — the user can see the file they were on. The action button takes them to `/sign-in?return_to=<current>`; after re-auth they land back exactly where they were.
4. WOPI editor iframes get a `postMessage('reauth-required')`; sheet/document show their own "Reconnect" toast and pause autosave until the iframe parent confirms re-auth.

**Keyboard.** `Shift-Cmd-Q` from anywhere; `Esc` from the avatar menu to close.

**Copy.**
- Menu item: **"Sign out"** + chord chip.
- Expiry toast: **"Signed out for security."** + action **"Sign back in"**.

**Polish notes.**
- No "Are you sure you want to sign out?" modal. Sign out is reversible — sign back in. Modals are reserved for actually-destructive operations.
- On expiry, don't blank the screen. The visible content stays; only writes are blocked.

**Edge cases.**
- User had an unsaved draft in an editor iframe: the iframe shows its own "Couldn't save" banner. Drive's job is to surface re-auth; the editor's job is to retry once auth is back.
- Tab was backgrounded for a long time: BFCache may serve a stale page. On `pageshow` event, ping `/api/me` and if 401, fire the expiry flow.

**Success criteria.**
- Sign-out invalidates the server-side session (next request 401).
- Expiry flow preserves view location and editor session continuity.

---

## 4 — Upload (button + drag-drop + folder)

**Goal.** User puts one or many files (or a folder tree) into Drive at a given location.

**Trigger.** Three entry points:
- **Button** — toolbar **Upload** button.
- **Drag-drop** — files from OS dragged onto Drive window.
- **Folder upload** — toolbar overflow → **Upload folder**, or Cmd-Shift-U on hover.

**Happy path — button.**

1. Click Upload → native file picker opens, multi-select allowed, all types accepted.
2. User picks one or more files → picker closes.
3. Files appear immediately in the current folder list as ghost rows: filename, `--fg-muted` size estimate, and a thin determinate progress bar across the bottom of the row in `--accent`. The row's icon shows a small upload-cloud overlay.
4. Each file streams via multipart to `POST /api/files?parent=<folder_id>`. On 201, the ghost row becomes a real row: progress bar fades out, upload-cloud overlay swaps for the file's type icon (resolved from sniffed mime).
5. On all uploads complete, a single sonner toast: **"Uploaded N files."** with action **"Show"** (no-op if user is already viewing them; scrolls them into view otherwise).

**Happy path — drag-drop.**

1. User drags from OS. See flow 1 for the drop-zone visual.
2. On drop: files queue identically to the button path. The drop card animates out (120 ms ease-out, opacity + scale 0.96).

**Happy path — folder upload.**

1. Native picker with `webkitdirectory` set, single root selectable.
2. Drive walks the tree and creates the implied folder structure server-side first (one `POST /api/folders/batch` with the relative paths), then uploads files concurrently (max 4 in flight; queue the rest).
3. The current folder list shows the first-level children immediately as ghost rows; nested children are visible if the user navigates into a subfolder during upload.

**Keyboard.** `U` opens the file picker. `Cmd-Shift-U` opens the folder picker. `Esc` cancels the *next* upload that hasn't started (in-flight uploads keep going); a second `Esc` cancels all queued.

**Copy.**
- Button: **"Upload"** + chord `U`.
- Toast (all done): **"Uploaded N files."** / **"Uploaded 1 file."**
- Toast (partial fail): **"Uploaded 7 of 10 files. 3 failed."** + action **"Retry failed"**.
- Per-row error tooltip: see edge cases.

**Polish notes.**
- Concurrent uploads cap at 4 (configurable). Queue UI: rows render in queue order with cap-aware progress.
- The ghost row appears in < 60 ms — instant feedback before the first byte goes to the server.
- Progress is *real* not faked — the bar reflects bytes acknowledged by the server (XHR `progress` event or fetch streams). Never animate from 0 → 100 over arbitrary time.
- A row that's been done for 800 ms loses its border tint; it should feel like it's "settled into the list".
- One toast per batch, not per file. Linear voice: terse, declarative.

**Edge cases.**

| Failure | Inline | Toast | Server |
|---|---|---|---|
| File > size cap | Row turns `--danger-muted`, tooltip: **"Too large. Max 100 MB."** | Per-batch summary | 413 |
| MIME sniff rejects file (HTML/exec on app origin) | Row error tooltip: **"This file type isn't allowed."** | Per-batch summary | 415 |
| Name collision in target folder | Row tooltip with two actions: **Replace** / **Keep both** | — | 409 with conflict ID |
| Network drop mid-upload | Row pauses with retry icon; tooltip: **"Paused. Retrying…"** | Only if all fail | — |
| Antivirus rejects (if enabled) | Row turns danger, tooltip: **"Couldn't upload — security scan blocked this file."** | Counts in batch toast | 422 |
| Storage quota exceeded | First failing row tooltip: **"You're out of space. Delete files or contact admin."** | Block-all toast | 507 |

**Success criteria.**
- 100 MB file uploads with real progress, no UI freeze.
- 100-file batch finishes cleanly with one toast.
- Folder upload creates the right tree on a fresh root.

---

## 5 — Browse: root → nested folders

**Goal.** User navigates from root into nested folders, sees breadcrumbs, can climb back up.

**Trigger.** Click a folder row, double-click in icon view, or `Enter` on a focused folder.

**Happy path.**

1. Single click selects the row (`--bg-selected`); double-click opens it. Power-user shortcut: `Cmd-Down` to enter the focused folder, `Cmd-Up` to go to parent.
2. URL updates: `/folders/<folder_id>` (history pushState). Back button works.
3. Header above the list shows breadcrumbs: `Home › Reports › 2026 › Q2`. Each segment is clickable; current segment is in default-fg weight, others muted. Long paths truncate the middle with **…**; hover shows a tooltip with the full path.
4. Sidebar reflects depth: the current folder's chain is expanded with disclosure chevrons.
5. Below the breadcrumbs: optional view toggle (list/grid/gallery — list is default), sort header, and a thin separator. Then the file list.

**Keyboard.**
- `↑` / `↓` move row focus.
- `Enter` open focused.
- `Cmd-Up` parent folder.
- `Backspace` (when nothing selected for delete) is **not** parent — Backspace is delete (Finder convention is wrong for a web app; Linear/Notion convention wins).
- `Home`/`End` jump to first/last.
- Letter keys jump to first row starting with that letter (sticky for 1 second).

**Copy.**
- Breadcrumb root: always **"Home"** (not "Root", not "My Drive").
- Empty folder: **"This folder is empty."** + small muted **"Drop files to add."** No button — uploading via drop or `U` is enough.

**Polish notes.**
- Navigating into a folder uses a 120 ms cross-fade, not a slide. Slides only when there's a clear left/right metaphor (mobile only).
- Pre-fetch on hover (>100 ms) — flow 11 covers this in detail.
- Tabular numerals on the Size column. Right-aligned. Always.
- Selected-row hover combines `--bg-selected` and a slightly stronger left-edge accent stripe — Linear pattern.

**Edge cases.**
- Folder was deleted by another tab while you were viewing it: on next focus, refresh shows the missing folder, route falls back to parent with a toast **"This folder no longer exists."**
- Network slow → skeleton rows (8 of them, 32 px tall, animated shimmer in `--bg-subtle` → `--bg-hover` over 1.2 s).
- Very deep path (10+ levels): breadcrumbs become `Home › … › Q2` with the **…** expandable into a dropdown of intermediate levels.

**Success criteria.**
- Sub-100 ms render when navigating into a pre-fetched folder.
- Browser back/forward works correctly across all nav.
- 10k items in a folder list renders smoothly (virtualised list).

---

## 6 — Open file in editor (WOPI)

**Goal.** User clicks a `.xlsx` or `.docx`, the right editor opens in a new tab (or same tab), edits sync to Drive, save round-trips through WOPI.

**Trigger.** Double-click on a file row, `Enter` on focus, or context-menu **Open**. Files with a registered editor open in the editor; others trigger flow 16 (download).

**Happy path.**

1. User double-clicks `Budget Q2.xlsx`. The row's icon shows a brief pulse (1 cycle, 200 ms, snappy).
2. Drive calls `GET /api/files/<id>/open` → server returns `{editor_app: "sheet", entry_url: "https://sheet.<host>/wopi/editor?WOPISrc=<...>", access_token: "<jwt>", access_token_ttl: <ms>}`.
3. Drive's client opens a new tab to `entry_url` and POSTs (programmatically — see WOPI brief §6 "post tokens into iframe, never GET") the access token. **Open in same tab** is also offered via the **Open ▾** split button.
4. Sheet's `/hosting/discovery` advertised this `urlsrc`; sheet's editor entry route now hits Drive's WOPI host: `GET /wopi/files/<id>?access_token=...` (CheckFileInfo) → `GET /wopi/files/<id>/contents` (GetFile) → starts ExcelJS in a worker → renders. Sheet acquires `Lock` on first user edit and runs the 10-min refresh loop until `beforeunload`.
5. From Drive's side, the file row now shows a small **"Open"** badge in `--accent` (subtle, not flashing) indicating an active session. Hover reveals **"Editing — open since 14:32"**.
6. Saves are invisible to the Drive UI; the row's `Modified` column updates lazily on next list-refresh (e.g. on focus return).

**Keyboard.**
- `Enter` opens in new tab (default).
- `Shift-Enter` opens in same tab.
- `Cmd-Enter` opens in a new tab but in **read-only** mode (no lock acquired; useful for "just look").

**Copy.**
- Split button: **"Open"** / **"Open in this tab"** / **"Open as read-only"**.
- Row badge: **"Open"** (tooltip: **"Editing — open since 14:32"**).
- Already-locked-by-someone toast (if multi-user later): **"Someone else is editing this. You can still view."** with action **"Open read-only"**.

**Polish notes.**
- The new-tab open uses `window.open(entry_url, '_blank')` triggered by the click event (browsers require user gesture). The token POST happens in the *new* tab on load; Drive passes the token via the launch URL's query string for the new tab to consume on `DOMContentLoaded`, then immediately removes it from `history.replaceState`.
- WOPI's "POST into JS-created iframe" rule applies *within* sheet's editor entry route, not Drive → sheet (the sheet route does the dance to defeat bfcache).
- Token TTL is 10 min; the editor's WOPI client transparently refreshes via `CheckFileInfo` near expiry — Drive's host re-issues a fresh token within the same cookie session.
- The pulse animation on double-click is real-time feedback that the system saw the action; no spinner.

**Edge cases.**
- File type has no editor: Open is replaced with **Download** in the menu; double-click triggers download (flow 16).
- WOPI session lock conflict (file locked by Office/Collabora, not us): we 409 with `X-WOPI-Lock` set; sheet shows its own banner.
- Network error opening the new tab: Drive's UI shows toast **"Couldn't open. Try again?"**; offers a retry that re-issues the launch URL.
- User's session expires while the editor tab is open: see flow 3 — editor pauses autosave, Drive surfaces re-auth, on success the editor resumes.

**Success criteria.**
- Round-trip works end-to-end against a real sheet/ build and a real document/ build.
- Lock acquire + refresh + release cycle verified in an e2e test.
- 10-min token expiry triggers a transparent refresh without interrupting edits.

---

## 7 — Rename file or folder

**Goal.** User renames an item in place; the new name validates and persists.

**Trigger.** `F2` (Windows convention) **or** `Enter` on focus (when focus matches double-click semantics, Enter renames in list view — Finder pattern is **return** which we match), **or** right-click → **Rename**, **or** click the name twice slowly (slow-double = rename, fast-double = open).

**Happy path.**

1. Row enters edit mode: the name cell becomes an inline text input pre-filled with the current name minus extension. The extension sits next to the input in `--fg-muted`, non-editable.
2. The name (sans extension) is auto-selected so the user can type immediately to replace, or hit `End` to position after.
3. User types. Validation runs live (see edge cases). The input border tints `--danger` on invalid.
4. `Enter` commits, `Esc` cancels. On commit: optimistic update — the row label changes immediately; `PATCH /api/files/<id> {name}` fires in background.
5. On 200: row settles; no toast for individual renames.
6. On 409 (name collision in same folder): the row reverts with a 1-cycle shake (8 px, 200 ms), the cell stays in edit mode, the helper line shows: **"Already a file with that name."**

**Keyboard.**
- `F2` enter rename.
- `Enter` commit. `Esc` cancel.
- `Tab` commit and start rename on next row (power-user batch rename).

**Copy.**
- Helper-line errors:
  - **"Name can't be empty."**
  - **"Already a file with that name."**
  - **"That name has characters we can't store: `\\ / ? *` (or others). Try removing them."**
  - **"Names can be at most 255 characters."**

**Polish notes.**
- Optimistic. The new name shows instantly; rollback is gentle.
- Auto-select-without-extension is the Finder/Notion convention — non-negotiable. People rename "report" parts, not ".docx" parts.
- Don't ever show a modal for rename. It's an inline edit. Always.
- The extension stays muted so it's clear it's separate but still visible.

**Edge cases.**
- Extension change: if user manually types a new extension (they had to click into the muted area or use End), confirm with a small inline check: **"Change extension to `.txt`?"** with **Yes** / **Keep `.docx`** buttons in muted-then-accent styling. This matches macOS Finder.
- Name with disallowed chars: live-strip OR show error — choice: strip silently is hostile; we show the error and let them fix it.
- Folder rename: same flow; the URL doesn't change because folders are addressed by ID, not name.

**Success criteria.**
- Rename is < 100 ms perceived (optimistic) and < 200 ms persisted.
- Collision flow leaves the user in a recoverable state (still in edit mode).

---

## 8 — Create new folder

**Goal.** User creates a new folder in the current location and gives it a name.

**Trigger.** Toolbar **New ▾** menu → **Folder** (with shortcut `Cmd-Shift-N`). Or right-click empty area → **New folder**.

**Happy path.**

1. A new row appears at the top of the list (or wherever sort dictates) titled **"Untitled folder"**, immediately in rename mode (flow 7).
2. The name is fully selected (folders don't have extensions, so no clever sub-selection needed).
3. User types name, hits Enter. Optimistic: folder is immediately addressable in the UI; `POST /api/folders` runs in background.
4. On 200: row settles. On error: row removed, toast **"Couldn't create folder. Try again?"**

**Keyboard.** `Cmd-Shift-N`. `Esc` to cancel an in-progress creation removes the row.

**Copy.**
- Default name: **"Untitled folder"** (lowercase 'f', matches Apple voice).
- Error toast: **"Couldn't create folder."** + action **"Try again"**.

**Polish notes.**
- Creating from a context-menu right-click puts the new folder at the cursor location in icon view, at sort-determined position in list view.
- The "Untitled folder" name auto-disambiguates ("Untitled folder", "Untitled folder 2", "Untitled folder 3" — same pattern Finder uses, with a space before the digit).

**Edge cases.**
- User immediately hits Enter without typing: folder is created as "Untitled folder" (or "Untitled folder N").
- User hits Esc: row removed, no server call.
- Storage doesn't support empty folders (S3-style): the folder is virtual until it contains a file; the UI still shows it. See `03-storage.md` §7 path/key notes.

**Success criteria.**
- New folder is immediately navigable.
- Disambiguation handles up to 100 conflicts gracefully.

---

## 9 — Move (drag + "Move to…" command)

**Goal.** User moves one or more items into a different folder.

**Trigger.** Three paths:
- **Drag** — drag selected items onto a folder row in the list, a folder in the sidebar, or a breadcrumb segment.
- **Command** — right-click → **Move to…** opens a folder picker.
- **Keyboard** — selection then `Cmd-Shift-M` opens the same picker.

**Happy path — drag.**

1. User starts dragging a selected file. Cursor shows the file icon + a stack badge ("3" for multi-select) anchored at the cursor.
2. Hover over a folder for >250 ms → folder row tints `--accent-muted`, slight inset shadow ("this is the target"). The cursor badge swaps to a `move` glyph.
3. Drop: rows fade out from current location (200 ms ease-out), toast appears top-right: **"Moved 3 items to *Reports*."** + action **"Undo"** (8 s lifetime).
4. Server-side `POST /api/files/move {ids, dest_folder_id}` runs in background. On success, no further UI. On failure, rows fade back in at original location with toast **"Couldn't move. Try again?"** + action **"Retry"**.

**Happy path — command picker.**

1. Picker modal opens (Radix Dialog, `--shadow-xl`, slide-up 200 ms): search box at top, folder tree below. Type to filter folders by path. `↑↓` navigate, `Enter` select, `Esc` cancel.
2. On select: same animation + toast as drag.

**Keyboard.**
- Selection → `Cmd-Shift-M` opens picker.
- In picker: `↑↓`, `Enter`, `Esc`.

**Copy.**
- Picker title: **"Move to…"**.
- Picker placeholder: **"Search folders"**.
- Toast: **"Moved 3 items to *Reports*."** + **Undo**.
- Toast (single): **"Moved *Budget Q2.xlsx* to *Reports*."** + **Undo**.
- Error toast: **"Couldn't move."** + **"Retry"**.

**Polish notes.**
- Drag spring on cursor badge: `{stiffness: 700, damping: 30}` — very snappy, follows the cursor without lag.
- Spring-loaded folders (hover over a folder for >700 ms during drag) — open into that folder so the user can drop deeper. macOS pattern, table stakes for a Finder-grade feel.
- Sidebar folder targets get the same hover-tint.
- Undo: optimistic rollback. If the 8 s lifetime elapses, the move is permanent; the server move was already committed.

**Edge cases.**
- Drop into current folder: no-op, no toast, no animation.
- Drop into a subfolder of the selection (folder onto itself): refuse — cursor badge shows `not-allowed`, no drop accepted.
- Name collision in destination: per-item conflict modal (similar to upload §4): **Replace** / **Keep both** / **Skip**, with **Apply to all** checkbox.
- Permissions deny (multi-user later): toast **"You can't move into *Reports*."**

**Success criteria.**
- Drag-drop works between list view, sidebar, and breadcrumbs.
- Undo restores within 8 s.
- Spring-loaded folders activate at 700 ms.

---

## 10 — Delete → trash

**Goal.** User sends one or more items to the trash, recoverable for 30 days.

**Trigger.** Selection then `Backspace` or `Delete`, **or** right-click → **Move to trash**, **or** toolbar trash icon.

**Happy path.**

1. Rows fade out (200 ms, opacity + 4 px translate-up), reflowing the list below.
2. Toast: **"Moved 3 items to trash."** + action **"Undo"** (8 s).
3. Server-side `POST /api/files/trash {ids}` runs in background. Soft-delete: items remain in storage with a `trashed_at` timestamp; `parent_folder_id` saved so restore can return them home.
4. On undo: rows fade back in; `POST /api/files/restore`.

**Keyboard.** `Backspace` or `Delete` on focused or selected. Both work for parity with Mac (Backspace) and Windows (Delete).

**Copy.**
- Toast (multi): **"Moved 3 items to trash."** + **Undo**.
- Toast (single): **"Moved *Budget Q2.xlsx* to trash."** + **Undo**.
- Toast (some not deletable, e.g. permissions): **"Moved 7 of 10 items to trash. 3 couldn't be moved."** + **Undo** (which undoes only the 7 that succeeded).

**Polish notes.**
- **No confirmation modal** for sending to trash. Trash is reversible (30 days). Modals are reserved for permanent destruction (flow 11).
- The fade animation is the confirmation — the user sees their action take effect.
- If user has `Cmd` held while deleting, that's still trash — `Shift-Cmd-Delete` is for permanent delete (matches Finder).

**Edge cases.**
- Trying to trash a file that's currently open in an editor (we hold a WOPI lock): refuse, toast **"Can't move *Budget Q2.xlsx* to trash — it's open in an editor."** with action **"Show file"**.
- Storage quota: trashed items still count toward quota. Surface this on the empty trash CTA: **"Trash uses 1.2 GB. Empty to reclaim space."** in flow 11.

**Success criteria.**
- Trash is reversible via toast within 8 s and via Trash sidebar within 30 days.
- Editor-locked files cannot be trashed.

---

## 11 — Restore from trash / empty trash

**Goal.** User views trashed items, restores one or many to their original location, or empties the trash permanently.

**Trigger.** Sidebar → **Trash** (Lucide `trash-2` glyph, item-count badge in muted text on the right).

**Happy path — view & restore.**

1. Sidebar **Trash** activates → main pane shows trashed items with a header strip: **"Trash"** title, **"Items in trash for more than 30 days are deleted automatically."** muted helper, two buttons: **Empty Trash** (danger-tinted ghost button) and the standard view-mode toggle.
2. Each row shows the file/folder, original location ("from *Reports / Q2*"), and how long ago it was trashed ("3 days ago").
3. Select rows → right-click → **Put back** restores them to their original `parent_folder_id`. If the original parent no longer exists (also in trash or deleted), restore to **Home** with a sub-text in the toast.
4. Toast: **"Restored 3 items."** + action **"Show"** (navigates to the parent of the first restored item).

**Happy path — empty trash.**

1. Click **Empty Trash** → modal (Radix Dialog, `--shadow-xl`):
   - Title: **"Empty Trash?"**
   - Body: **"This will permanently delete 23 items. You can't undo this."**
   - Buttons: **Cancel** (ghost) / **Empty Trash** (danger fill).
2. Confirm → items are hard-deleted. Toast: **"Trash emptied."** No undo button — this is destructive by design.

**Keyboard.**
- `Cmd-Z` undoes the most recent trash-move (within 8 s — same as the post-delete toast).
- In Trash view: `R` restores selection. `Shift-Cmd-Delete` opens the Empty Trash confirm.

**Copy.**
- Header subtitle: **"Items in trash for more than 30 days are deleted automatically."**
- Empty Trash modal:
  - Title: **"Empty Trash?"**
  - Body: **"This will permanently delete 23 items. You can't undo this."**
  - Buttons: **"Cancel"** / **"Empty Trash"**.
- Restore-to-fallback toast: **"Restored 3 items to *Home* (original folder no longer exists)."**

**Polish notes.**
- Empty Trash modal is **one** of the very few flows that gets a modal. Permanent destruction earns the friction.
- The Empty Trash modal's primary action is in `--danger` fill, not the standard `--accent`. Apple's convention.
- 30-day auto-delete runs server-side daily; no UI noise.

**Edge cases.**
- Trashed file is referenced by an active share-link: empty-trash deletes the share-link too (broken link → 404 for recipients). The modal warns: **"This will also delete 2 share-links."** Inline list of affected links would be over-engineering at v0.
- A folder in trash has children also in trash: restoring the parent restores all children to their original locations (independently — each child has its own `parent_folder_id`).

**Success criteria.**
- Restore returns items to original parent or Home fallback.
- Empty Trash purges from storage adapter, frees quota, kills associated share-links.

---

## 12 — Multi-select + bulk actions

**Goal.** User selects multiple items and applies an action across all of them.

**Trigger.** Within any file list:
- **Cmd-click** toggles a single item's selection.
- **Shift-click** range-selects from anchor to clicked.
- **Cmd-A** selects everything in current folder.
- **Rubber-band** drag (icon/grid view) lasso-selects.

**Happy path.**

1. Selection visual: row gets `--bg-selected`, a 2 px left-edge `--accent` stripe, and (when row is focused) the focus ring.
2. As soon as ≥2 items are selected, a **selection bar** slides up from the bottom of the main pane (Radix Toast position, but persistent — vaul-style drawer 200 ms ease-out, `--shadow-lg`). The bar shows: count (**"3 selected"**), then action chips: **Download**, **Move…**, **Share…**, **Trash**, separated, then **Clear** on the right.
3. Each action targets the selection. Trash uses flow 10. Move uses flow 9. Share uses flow 14. Download bundles selection as `.zip` (flow 16).
4. Clear (or `Esc`) dismisses selection and the bar.

**Keyboard.**
- `Cmd-A` select all.
- `Esc` clear selection.
- All the per-item shortcuts (`Backspace`, `Enter`, `F2`, `Cmd-Shift-M`) apply across selection where it makes sense.

**Copy.**
- Bar prefix: **"3 selected"** (always plural-aware: "1 selected", "2 selected", "127 selected").
- Action chips: **Download** · **Move…** · **Share…** · **Trash** · **Clear**.
- Trash-many confirmation: see flow 10 (no modal; just toast + undo).

**Polish notes.**
- Selection bar is one of the few places we use vibrancy: `backdrop-filter: saturate(180%) blur(20px)` over `rgba(255,255,255,0.7)` (or dark equivalent).
- Bar height matches the bottom safe-area; on slim windows, bar wraps into a more compact form (icon-only).
- Multi-select with mixed types (folders + files): actions that don't apply to all types are hidden, not greyed. (E.g. **Share** for folders behaves differently from files; first version keeps folders out of share, hides the chip if any folder is selected.)
- Rubber-band lasso is icon-view only; in list view, hold `Shift` and drag.

**Edge cases.**
- Selection across folder boundaries via Cmd-A only goes as deep as current folder (matches Finder).
- Hitting `Cmd-A` in a 10k-item folder is fast (selection is just a `Set<id>`, no DOM mutation per item — virtualised list).

**Success criteria.**
- Selection bar appears for any selection ≥ 2.
- Bulk trash → 1 toast (not N).

---

## 13 — Search (Cmd-K palette)

**Goal.** User finds a file or folder by typing, regardless of where they are.

**Trigger.** `Cmd-K` from anywhere. Also: clicking the **Search** input in the header (which actually opens the palette as a focused overlay; we don't have a separate input).

**Happy path.**

1. Palette opens centered, 600 px wide, `--radius-xl`, `--shadow-xl`, backdrop-filter blur over a subtle dim. cmdk under the hood.
2. Top: input with placeholder **"Search files or run a command…"** Lucide `search` glyph on the left.
3. As user types, two grouped result sections render below:
   - **Files** (top, up to 8) — file/folder hits, with their location path muted on the right.
   - **Commands** (below) — actions like **"New folder"**, **"Upload"**, **"Empty Trash"**, **"Sign out"**, **"Toggle theme"**. Each shows its shortcut chip aligned right.
4. `↑↓` navigate, `Enter` select, `Esc` close.
5. Selecting a file: navigates to and selects that file in its parent folder. Selecting a command: runs it.

**Keyboard.** `Cmd-K` open. Arrow keys navigate. `Enter` select. `Esc` close.

**Copy.**
- Placeholder: **"Search files or run a command…"**.
- Empty (no input): **"Start typing to search."** + below it the **Recent files** group (up to 5) and **Commands** group (up to 6).
- No results: **"No files match \"…\"."**.
- Loading: skeleton rows in the Files section (4 rows, 28 px each).

**Polish notes.**
- Open and close transitions: 150 ms `--ease-out` opacity + 4 px translate-Y. No bounce — calm not playful.
- Result rendering is debounced 80 ms — slow enough not to thrash, fast enough to feel live.
- Recent-files list persists in IndexedDB; survives reloads.
- Command names use sentence case ("New folder", not "New Folder").
- The palette is the safety net (commandment #7's commentary): every important action exists here, but the user should rarely need it because the chord chips elsewhere already advertised the shortcut.

**Edge cases.**
- Network slow → skeleton stays until results arrive; no spinner.
- Query > 200 chars: truncate input visually, query the trimmed value.
- Selected file is in trash: open in Trash view, not original folder.

**Success criteria.**
- Sub-100 ms keystroke-to-result render on cached file list.
- Cmd-K reachable from every page, including the empty state and the sign-in error state.

---

## 14 — Create share-link

**Goal.** User generates a shareable URL for a file (or folder) with optional password, expiry, and permission level.

**Trigger.** Right-click → **Share…**, or selection then **Share** action, or toolbar **Share** button when one item is focused. Shortcut: `Cmd-Shift-S`.

**Happy path.**

1. Modal opens (Radix Dialog, 480 px wide, `--shadow-xl`): title **"Share *Budget Q2.xlsx*"**.
2. The sharing URL is created **immediately** on modal open (a fresh 128-bit token, default perms: **view**, no password, expires 7 days from now). The URL appears in a read-only text field at the top, alongside a Lucide `copy` button.
3. Below the URL, three rows of controls:
   - **Who can…** — segmented control: **View** / **View + Download** / **Edit** (only shown if file type has an editor). Default: View. (Edit grants WOPI-write to the recipient through the share-link token; see auth brief §4.)
   - **Password** — toggle + input that appears when on. Strength indicator below. Optional.
   - **Expires** — toggle + date picker. Default toggled on at 7 days.
4. Any control change rotates the URL or updates the link metadata — debounced 400 ms, no Save button needed.
5. Existing share-links (if any for this file) listed at the bottom: each row shows the URL (truncated), perms badge, created date, optional password lock glyph, expiry. Each has a **…** menu: **Copy**, **Revoke**.

**Keyboard.**
- `Cmd-Shift-S` open.
- `Cmd-C` while URL is focused → copy.
- `Esc` close.

**Copy.**
- Modal title: **"Share *<filename>*"**.
- URL helper text (below the URL field, muted): **"Anyone with this link can view."** (updates as perms change: "…can edit", "…can view and download").
- Password section title: **"Require a password"**.
- Expiry section title: **"Expires"**.
- Copy button label (icon-only with tooltip): **"Copy link"**. Tooltip after click: **"Copied"** (revert 1 s).
- Revoke confirm (inline next to the row, not a modal): **"Revoke this link?"** with **Revoke** / **Cancel** inline buttons.

**Polish notes.**
- The URL appears **before** any user action, with sensible defaults. No "Generate link" button — that's an extra step Notion got rid of and we should too.
- Copy button click flashes the URL field with a 1-cycle `--bg-selected` (200 ms). Tactile feedback for "yes, I copied that".
- The strength indicator under password uses 4 colored bars (Tailwind-classic) but in our `--success/warning/danger` palette, not the cliché green-yellow-red.
- Closing the modal does **not** delete the link. Links persist until manually revoked or expired.

**Edge cases.**
- File is trashed or deleted while share modal is open: surface inline error **"This file no longer exists."** and close button only.
- User picks **Edit** perms for an editor-less file (e.g. PNG): the **Edit** segment is hidden — only View / View + Download.
- Backend rate-limit on link creation (e.g. 50/min/IP): error **"Try creating links a bit slower."**
- Password user enters violates length policy: inline error below input.

**Success criteria.**
- Link is live the moment the modal opens.
- Revoke is one click + one confirm.
- Editing perms/password/expiry never requires re-creating the link (same URL, mutable metadata).

---

## 15 — Recipient opens share-link

**Goal.** Someone with a share-link URL accesses the file, optionally entering a password, and downloads or opens it.

**Trigger.** Recipient pastes/clicks `https://drive.<host>/s/<token>`.

**Happy path — no password.**

1. Server validates token. Renders a minimal page: filename, size, **"Shared by *<owner name or 'someone'>*"**, primary action **Download** (or **Open in editor** if editable + recipient has perms).
2. If editable: **Open** button launches the WOPI flow with a share-link-scoped access token (same flow 6, with `view-only` enforced if perms say so).
3. If view-only and previewable (image, PDF): inline preview rendered from user-content origin.

**Happy path — with password.**

1. Server returns a password page: same minimal chrome, just **"This link requires a password."** with input + **Continue**.
2. On submit, server validates (constant-time compare), sets a short-lived cookie scoped to this share-token, redirects to the normal share view.
3. Repeated wrong passwords are rate-limited (same `tower_governor` rule as login: 10/min/IP).

**Keyboard.** `Enter` submits password.

**Copy.**
- View page title: **"Shared with you"** (or owner name if available).
- View page primary button: **"Download"** / **"Open"** / **"Open in editor"**.
- Password page: **"This link requires a password."** + **"Password"** input + **"Continue"**.
- Wrong password: **"Wrong password."** (same as flow 2 — consistency).
- Expired link: **"This link expired on 12 May."**
- Revoked link: **"This link is no longer active."**

**Polish notes.**
- The recipient's view is **deliberately stripped of Drive chrome** — no sidebar, no main app shell. Single centered card, marketing-quality. Recipients are not users of Drive; they should not get a hint there's a file manager behind the link.
- For previewable content (image/PDF), the preview is inline and big — the page feels like a "look at this" page, not a "download from a corporate portal" page.
- The little **"Powered by Casual Drive"** footer link is optional and disabled by default in self-host (operator opt-in).

**Edge cases.**
- Link points to a file that's been moved: still works (file_id is stable).
- File in trash: 404 with **"This link is no longer active."** (don't disclose trash semantics to recipients).
- Editor open in editable mode: recipient gets a WOPI access token; their edits go through PutFile like any other user; for v0 there's no per-user attribution on the file — the share-token consumer is treated as a single anonymous editor.

**Success criteria.**
- Recipient flow works end-to-end with no Drive account.
- Password + expiry + revoke all enforced.
- View-only enforcement: even if a recipient sniffs the WOPI token, the perms claim prevents `PutFile`.

---

## 16 — Download (single + selection-as-zip)

**Goal.** User downloads one file as-is, or a selection (including folders) bundled as a `.zip`.

**Trigger.**
- **Single** — right-click → **Download**, or selection of 1 then `Cmd-D`, or context-menu on a non-editable file (then `Enter`/double-click).
- **Selection** — multi-select (flow 12) then **Download** in the selection bar, or `Cmd-D`.

**Happy path — single.**

1. Click **Download** → browser starts download immediately. The URL is the user-content origin signed-URL (S3 presigned, or `/raw/<token>` for fs/memory).
2. `Content-Disposition: attachment; filename*=UTF-8''<encoded>` ensures correct filename + non-executable handling on the browser side.
3. No toast for single-file downloads — the browser's own download UI is the feedback.

**Happy path — selection / folder.**

1. Click **Download** → request `POST /api/files/zip {ids}` returns a job ID and a streaming URL.
2. UI shows a small persistent footer pill: **"Preparing zip… 12% (3 of 25 files)"** with cancel `×`. Progress is real (server reports per-file completion via SSE or chunked progress).
3. When ready, the browser download starts automatically (page invokes the URL programmatically — browsers allow this within the same gesture chain if the response comes within ~3 s, otherwise we show **"Download zip"** button and the user clicks).
4. Pill collapses 2 s after download starts. Single toast: **"Downloaded *Selection (25 files).zip*."**

**Keyboard.**
- `Cmd-D` download focused or selection.
- `Esc` cancels in-progress zip prep.

**Copy.**
- Pill (preparing): **"Preparing zip… N%"** + cancel.
- Pill (ready, gesture lost): **"Download zip"** button.
- Toast (single done): no toast.
- Toast (zip done): **"Downloaded *<zip name>*."**.
- Toast (zip cancelled): no toast (the cancel was the feedback).

**Polish notes.**
- Single download: zero ceremony. Just go.
- Zip download: the pill is one of the few persistent footer surfaces — restrained, dismissable, doesn't block content.
- Server streams the zip on the fly (no temp file). On the user-content origin. With `Content-Disposition: attachment`.
- Zip filename pattern: if all items share a parent folder → **"<parent name>.zip"**; otherwise **"Casual Drive ({YYYY-MM-DD HH:MM}).zip"** — Apple's "Archive.zip" with our brand and timestamp.

**Edge cases.**
- Selection contains 1 file: skip the zip path, download single (smart default).
- Zip exceeds a soft cap (e.g. 2 GB) or contains > 5,000 entries: warn first in a non-modal banner **"That's a large download (3.4 GB, 7,200 files). It may take a few minutes."** with **Continue** / **Cancel**.
- File becomes unavailable mid-zip (deleted, scan failed retroactively): skip + note in trailing toast **"Downloaded zip. 1 file was skipped."**
- Browser blocks auto-download (no user gesture in window): fall back to the "Download zip" button form.

**Success criteria.**
- Single download starts in < 200 ms.
- 1 GB zip download streams without server memory blowup.
- Cancel during zip prep is honoured promptly.

---

## What this doc deliberately doesn't cover (deferred)

The deferred set, in priority order for the next pass:

1. **Settings page** — change admin password, theme, default upload behaviour, share-link defaults, antivirus toggle, S3/MinIO credentials.
2. **Preview pane** — inline preview for images, PDFs, plain text, markdown. Selectable via toolbar; remembers state per user. (Quick-Look-like spacebar preview also goes here.)
3. **File properties dialog** — full metadata, sniffed type, hash, history of share-links, related editor sessions.
4. **Quota near full** — banner at 90%, modal at 100%.
5. **Virus scan rejection** — refined toast + admin notification path.
6. **Keyboard shortcut help** — `?` opens a Cmd-K-style cheat sheet.
7. **Theme toggle** — sun/moon glyph in sidebar avatar menu; flip animation 250 ms.
8. **Public landing** — when an unauthenticated visitor hits `/` directly (not via share-link), what they see. Likely a small "This is a private Casual Drive instance" card with a Sign in link.
9. **Sidebar customisation** — pin folders, reorder favourites.
10. **Bulk-move conflict resolver** — better UX than per-item dialogs for moves of 100+ items into a destination with collisions.
