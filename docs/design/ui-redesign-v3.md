# UI Redesign v3 — Fewer things, better placed, on glass

A **layout + information-architecture** rethink of the Doc-Hub SPA, layered on top of
[`ui-system.md`](./ui-system.md) (density, ink/paper + amber identity, compliance affordances — unchanged)
and [`ui-system-glass.md`](./ui-system-glass.md) (the glass material + Spotlight target — the finish).

This doc governs **what appears, where, and why**. It does not change the density scale, the
palette, the compliance semantics, or the glass tokens — it *removes, relocates, and consolidates*
the surfaces those systems render.

> North star: macOS / iOS / Spotlight / Things discipline. Nothing is on screen unless it earns its
> place. The document's identity is its **version chain, encryption state, and provenance** — not
> generic metadata. Everything else is progressively disclosed, keyboard-first, on frosted glass.

---

## 0. Executive summary — the top layout decisions

1. **⌘K Spotlight is the single search + command surface.** Delete the top-bar search input and its
   recents popover. Search, navigation, and *compliance quick-actions* (verify chain, sign, place
   hold, export bundle) all live in the glass Spotlight overlay.
2. **The top bar collapses to identity + two ambient status affordances.** Remove view toggle,
   density toggle, help button, and search from the top bar. What remains: logo, notifications
   (home only), encryption/key status, account. It becomes a thin glass rail, not a control strip.
3. **The vault table sheds three redundant columns** — Kind (duplicates the icon), Lock and
   Encryption (both say "encrypted", always, for every row). Encryption is *ambient* (sidebar footer,
   one place). Version becomes **conditional** (shown when compliance-significant). ~190px reclaimed.
4. **DetailsPanel collapses from a 3-tab drawer to one focused compliance card.** Info tab → removed
   (pre-known metadata). People/Share tab → removed (routes to the existing ShareDialog). What stays:
   a two-line proof summary (`Encrypted · v12 · ✓ Verified`) + "View history →".
5. **`/document/{id}/history` is the one canonical version-history home.** Full-width, archival-grade,
   holds banner, tamper alarm, verify + restore + export. The panel only ever *summarizes and links*
   to it. Three homes become one.
6. **Preview and Edit stay separate — they solve different jobs**, but the boundary is sharpened:
   single-click = lean glass Preview (content + proof summary, sidebar off by default); double-click /
   Open = fullscreen editor. On viewports < 800px, single-click routes straight to fullscreen.
7. **Coming-soon nav items and dead controls are deleted, not disabled.** Recent/Starred/Shared nav
   stubs, the no-op Star button, the Settings→Audit stub all go. AvatarStack and WorkspaceSwitcher are
   gated on whether multi-workspace / co-editing is actually live.
8. **Settings and Admin de-duplicate and re-scope.** Build info lives in Admin→System only. Retention
   and workspace policy move to Admin (admin-gated). Settings drops from 12 → 8 sections along a clean
   Personal / Workspace / Compliance axis; Admin from 9 → 6 tiles.

Net effect: the top bar goes from 4 controls to ambient status; the table from 8 columns to 5; the
details drawer from 3 tabs to 1 card; version history from 3 homes to 1. Fewer things, on glass.

---

## 1. Shell — sidebar + top bar + Spotlight

### 1.1 Information architecture — remove / relocate / keep

| Element | Verdict | Rationale (one line) |
|---|---|---|
| Top-bar search input + recents popover | **REMOVE** | Duplicates ⌘K with a slower, narrower, polled surface; ⌘K wins. |
| Top-bar view toggle (Grid/List) | **RELOCATE → Settings › Display + row context menu** | Set-once preference; does not earn premium top-bar space. |
| Top-bar density toggle (Comfortable/Compact) | **RELOCATE → Settings › Display** | Persisted preference, toggled rarely; belongs with theme. |
| Top-bar help button (?) | **REMOVE** | Third redundant help surface; keep the ⌘K "Keyboard shortcuts" action + `?` key. |
| Coming-soon nav (Recent, Starred, Shared) | **REMOVE** | Dead links with "soon" badges are visual debt; re-add when shipped. |
| AvatarStack (sidebar) | **REMOVE (gate on co-edit MVP)** | Premature if real-time co-edit isn't live; move to file row / details when it is. |
| WorkspaceSwitcher (sidebar) | **KEEP only if multi-workspace is live; else REMOVE** | Decorative in a single-workspace install; if kept, a compact `Workspace ⌄` dropdown. |
| Notifications bell | **KEEP, home-only, gated render** | Compliance cue, but ambient noise off the home tab; render only when `nav==="home"`, pause polling otherwise. |
| **+ New** button (sidebar) | **KEEP + mirror into ⌘K** | Primary ingest action earns the rail; also expose New/Upload as ⌘K quick-actions + ⌘N. |
| Encryption footer chip | **KEEP** | Load-bearing ambient trust cue; the *one* place encryption is stated. |
| AvatarRow + ThemeToggle (footer) | **KEEP** | Well-placed account + theme controls. |
| Section labels (LIBRARY / WORKSPACE / SYSTEM) | **KEEP** | Cheap, clear hierarchy. |
| CommandPalette (⌘K) | **KEEP + EXPAND to Spotlight** | Promote to the canonical search/nav/compliance-action surface (§1.3). |

### 1.2 Lean structure (glass)

```
┌─────────────────────────────────────────────────────────────────────────┐  ← --mat-thin glass,
│ ◧ Doc-Hub                                        🔔    🔒·key ⌄ schnsrw  │    --blur-chrome, edge-hi
├──────────────┬──────────────────────────────────────────────────────────┤    top bar 48px
│  SIDEBAR     │  Breadcrumb ▸ Project ▸ Folder              [+ New] [↑]    │  ← content toolbar (glass)
│  240px       │ ┌──────────────────────────────────────────────────────┐ │
│  --mat-thin  │ │  Name              Ver*  Status       Modified   Size │ │  header 36px, glass
│              │ ├──────────────────────────────────────────────────────┤ │
│ Personal     │ │ 📄 Contract.pdf          🔒✓ intact   2h ago    1.4M │ │  row 32px, near-solid
│  ─ Locker    │ │ 📄 Q3.xlsx         v3    ⚖ hold       1d ago     88K │ │  (--mat-thick, crisp)
│ My Drive     │ │ …                                                    │ │
│ Notes        │ └──────────────────────────────────────────────────────┘ │
│ ──────────   │                                                          │
│ Activity     │        ⌘K  →  Spotlight: search · go to · verify ·       │
│ Admin        │              sign · hold · export  (glass overlay)       │
│ ──────────   │                                                          │
│ Trash        │                                                          │
│ Settings     │                                                          │
│ ──────────   │                                                          │
│ schnsrw  🌓  │                                                          │
│ 🔒 Encrypted · AES-256-GCM   ← always-on glowing glass pill              │
└──────────────┴──────────────────────────────────────────────────────────┘
   * Ver column renders only when compliance-significant (see §2).
```

- Top bar becomes **ambient status only**: no controls that mutate a preference. Materials:
  `--mat-thin` + `--blur-chrome`, `--edge-hi`; `--shadow-sm` appears only when content scrolls under.
- Bell is a glass icon button with the amber badge; rendered only on `home`. Encryption/key status is
  a quiet `lock`/`key` glyph → opens a read-only key-status popover (mirrors Admin, never key material).

### 1.3 Spotlight — the command surface (⌘K)

Per glass §5: centered ~640px, `--mat-thick`, `--blur-overlay`, `--shadow-overlay`, `--radius-xl`,
spring entrance, dimmed+blurred scrim. Groups:

```
┌ ⌘K ───────────────────────────────────────────────────────────── glass ┐
│  ⌕  verify chain                                                        │  ← big translucent input
├─────────────────────────────────────────────────────────────────────── ┤
│  ACTIONS (context-aware — only when a doc/version is selected)          │
│   ✓  Verify chain — Master-Agreement.pdf              ⌘⇧V                │  ← amber-wash row,
│   ⧉  Export offline-verifiable bundle                                   │    --accent-glow on icon
│   ⚖  Place legal hold…                                                  │
│   ✎  Sign this version…                                                 │
│  GO TO                                                                   │
│   ▸  Activity   ▸ Trash   ▸ Settings   ▸ Admin                          │
│  QUICK CREATE                                                            │
│   ＋ New document   ⌘N     ＋ New folder   ⌘⇧N     ↑ Upload             │
│  DOCUMENTS · FOLDERS · NOTES   (Tantivy full-text, snippet + highlight) │
│  AI · read-only  (sparkles) — never mutates                            │
└─────────────────────────────────────────────────────────────────────── ┘
```

- **This is the search bar.** There is no other. Compliance actions are context-aware (present only
  when a file/version is selected), so the shell stays static and calm.

### 1.4 Compliance/security in the new shell

- Encryption: **one** ambient statement — the sidebar footer glass pill. Removed from every row.
- Key/rotation status: quiet `key` glyph in the top bar → read-only popover.
- Verify / sign / hold / export: reachable in one keystroke via Spotlight, plus their canonical homes
  (history route, provenance card). Never buried, never cluttering the resting shell.

---

## 2. Vault table — the document list

### 2.1 Information architecture — remove / relocate / keep

| Column | Verdict | Rationale |
|---|---|---|
| Checkbox (select) | **KEEP** | Bulk-action enabler; keyboard + roving tabindex. Consider 0.3 rest opacity (Finder). |
| Icon + Name | **KEEP** | Primary identifier; icon disambiguates type. File-viewing presence dot stays. |
| Kind label ("Document"/"Spreadsheet") | **REMOVE** | Duplicates the icon; ~70px of no new information. Move to icon tooltip if a11y needs it. |
| Version (`v12`) | **RELOCATE → conditional** | Show only when `hold │ retention_due │ requires_signature │ versions>1`; else omit. Full chain is one click away. |
| Modified (relative) | **KEEP** | Primary sort + recency scan for audit. |
| Lock icon | **REMOVE** | Encryption is ambient + universal; stated once in the footer, not per row. |
| Encryption chip ("AES-256-GCM") | **REMOVE** | Second element saying "encrypted"; pure clutter. Per-doc cipher detail belongs in the compliance card. |
| Status cluster (verify / hold / signed) | **KEEP** | Load-bearing compliance state; **this** is what the row must carry, not encryption. |
| Kebab actions | **KEEP + re-order** | Hover/focus reveal. Hierarchy: Open, History (compliance), — , Rename, Share, Download, Trash. |

Result: **8 columns → 5** (`select · name · version* · status · modified · size`), ~190px reclaimed,
32px row preserved, legible at narrow widths and deep nesting.

### 2.2 Lean structure (glass)

```
 ┌──┬───────────────────────────┬──────┬───────────────┬──────────┬────────┐
 │▢ │ 📄 Master-Agreement.pdf    │ v12* │ ✓ intact      │ 2h ago   │ 1.4 MB │  row 32px, near-solid
 └──┴───────────────────────────┴──────┴───────────────┴──────────┴────────┘  (--mat-thick, crisp)
   ▲ hover → soft lift (shadow + 1px), NOT a color swap · actions fade in right
   ▲ selected → amber-wash + --edge-hi + 2px left amber rule
   * version cell present only when the row carries a compliance signal
```

- Header row: `--mat-thin` glass. Data rows on near-solid `--mat-thick` so dense text stays AA-crisp
  (glass never costs readability — glass §5). Hover = whisper of elevation; selection = amber wash.

### 2.3 Compliance in the table

- The status cluster is the row's compliance payload: `shield-check`/`shield-alert` (intact/tamper),
  `gavel` (hold), `badge-check` (signed) — icon **and** label, never color-only.
- Encryption is intentionally *absent* here (ambient elsewhere). Tamper remains an alarm, not a tint.

---

## 3. Preview vs. Edit — the two document surfaces

### 3.1 Decisive answer to the redundancy

**Keep both. They are different jobs.** Preview = scan/inspect content without committing to the
editor (Finder/Photos convention). Fullscreen = focused authoring + co-edit. But sharpen the boundary:

- **Single-click → lean glass Preview.** Sidebar **off by default**; stage shows content, a one-line
  proof summary sits under the title, "Details" and "History" are opt-in.
- **Double-click / Open / ⌘↩ → fullscreen editor.**
- **Viewport < 800px → single-click routes straight to fullscreen** (the two-column modal breaks on
  narrow screens; don't render a crippled version).
- Transition between them stays seamless (Expand button, no reload).

### 3.2 Preview modal — remove / relocate / keep

| Element | Verdict | Rationale |
|---|---|---|
| Preview stage (content) | **KEEP** | The reason to single-click. PDF.js / image / text renderers land here. |
| Always-on 320px Details sidebar | **RELOCATE → opt-in** | Default off; the modal is preview + proof summary. "Details" button opens the panel on demand. |
| Star button (no-op) | **REMOVE** | Dead UI; re-add via kebab when favoriting ships. |
| Close (×) button | **REMOVE** | Esc + scrim-click already close; the × is redundant chrome. |
| Download button | **KEEP** | Load-bearing, also in fullscreen toolbar. |
| Expand (fullscreen) button | **KEEP** | The bridge to the editor. |
| Share button (action bar) | **RELOCATE → ShareDialog** | Share is one action, not co-equal to Open; a header/kebab entry opens the existing dialog. |
| Open (primary) | **KEEP** | The one primary action per surface. |
| Prev/Next nav arrows | **KEEP** | Photos-class flip-through; keyboard ←/→ advertised. |

### 3.3 Preview — lean structure (glass)

```
┌ Preview ─────────────────────────────────────────────  ↑Download  ⤢ Expand ┐  ← --blur-overlay glass,
│                                                                             │    spring entrance, scrim
│   ‹                                                                     ›   │
│                        [ content stage ]                                    │
│                                                                             │
│   📄 Master-Agreement.pdf   ·   PDF · v12                                    │  ← title + proof one-liner
│   🔒 Encrypted · v12 · ✓ Verified                          [ Details ]      │  ← compliance summary card
│                                                                             │
│                                              [ Open in editor ]  (primary)  │
└──────────────────────────────────────────────────────────────────────────── ┘
```

- The proof one-liner (`Encrypted · v12 · ✓ Verified`) replaces the always-on Info tab: the three
  facts that matter for a records tool, stated inline. "Details" discloses the rest on demand.

### 3.4 Fullscreen editor header — remove / relocate / keep

The current header is already lean and appropriate. Deltas:

| Element | Verdict | Rationale |
|---|---|---|
| Back, editable filename, version chip, save-status pill, collab presence, file-presence stack | **KEEP** | Each earns its place; save-status auto-hides; presence is a live signal. |
| Details button | **KEEP → opens single-card drawer** | Drawer now shows *version history only* (§4), not a 3-tab grab-bag. |
| Share button | **KEEP (move to kebab on < 800px)** | Common enough to earn a button on desktop; collapse to kebab when the header tightens. |
| Kebab (⋯) | **KEEP** | Rename, Download, Trash (if not held), History, Permissions. |

---

## 4. DetailsPanel + Version history — the compliance core

### 4.1 Decisive answer to the redundancy

**`/document/{id}/history` is the single canonical version-history home.** Full-width, deep-linkable,
archival-grade legibility, room for the holds banner + tamper alarm + verify/restore/export. The
DetailsPanel and the editor drawer only ever render a **summary that links to it**. Activity stays a
*separate, hub-wide* audit log (§5) and cross-links into the per-document history.

### 4.2 DetailsPanel — collapse 3 tabs → 1 card

| Element | Verdict | Rationale |
|---|---|---|
| Info tab (type, size, owner, created, modified, location, content-type) | **REMOVE** | Pre-known (row + header) or static; not the document's real identity. Optional right-click "Details…" modal if ever needed. |
| People tab (share links) | **REMOVE → ShareDialog** | Sharing is a toolbar action with an existing dedicated dialog; a sidebar tab is a detour. |
| History tab (version chain, panel variant) | **REPLACE with summary** | 360px is claustrophobic for a 12-node hash chain; show `v12 · 11 prior · ✓ intact` + "View history →". |
| "Verify chain" button (panel) | **REMOVE from panel** | One home: the full route (+ ⌘⇧V in Spotlight). No duplicated primary action. |
| Per-file tamper alarm (panel) | **CONSOLIDATE → full route + Activity link** | The alarm lives on the history route and links to the Activity event that broke the chain. |

Result: DetailsPanel becomes a **single, un-tabbed compliance card**:

```
┌ Master-Agreement.pdf ─────────────────────────────────────────────── ✕ ┐  ← --mat-regular glass drawer
│  🔒 Encrypted at rest · AES-256-GCM                                     │
│  ⛓ Version v12 · 11 prior · ✓ Verified                                 │
│  ───────────────────────────────────────────────────────────────────   │
│  [ View full history → ]              [ Share ]                          │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Version history — the canonical full route (glass)

`/document/{id}/history` keeps the ui-system §7.3 anatomy, elevated to glass §7:

```
┌ Version history — Master-Agreement.pdf ───────────────── [Verify chain] ⌘⇧V ┐
│  ⚖ Legal hold · since 2026-03-11 · delete/tombstone/purge blocked  [audit] │  ← holds banner (route only)
│  ●  v12  current            2h ago · schnsrw   reason "signed final"        │  ← glass nodes, subtle spine
│  │   content_hash 9f3a…c1 ⧉   ✓ link intact                    [Restore ⤴] │
│  ┿  ○ v11 …                                                                 │
│  ⛓✗ v7 → v6  LINK BROKEN → audit #a91f…                          [Details] │  ← --accent-glow tamper banner
│  ─────────────────────────────────────────────────────────────────────     │
│  Chain: 12 versions · ✓ 11 verified · ✗ 1 broken · Append-only   [Export ⧉]│
└─────────────────────────────────────────────────────────────────────────── ┘
```

- Holds banner and tamper alarm render **here** (never in the 360px summary). Tamper links to the
  Activity event that broke the chain — one source of truth, cross-referenced.

---

## 5. Activity, Settings, Admin

### 5.1 Activity — keep, clarify scope

- **KEEP** the full-page route; it is the hub-wide, append-only audit log — *not* per-document history.
- **RELABEL** sidebar item to `Audit` (or `Activity` with an "audit trail" subtitle) to end the
  ambiguity with version history.
- **CROSS-LINK**: a broken-chain alarm on `/document/{id}/history` links to the Activity event; an
  Activity version-change row links back to the document's history. Never merge the two.
- Glass: day-group headers on `--mat-thin`; rows near-solid; tamper alarm an `--accent-glow` banner.

### 5.2 Settings — 12 → 8 sections

| Section | Verdict | Rationale |
|---|---|---|
| Account, Notifications, Tokens/Sessions | **KEEP → group "Personal"** | Genuine per-user preferences. |
| **Display** (theme + view + density) | **NEW** | New home for the relocated view/density toggles (§1) beside theme. |
| Members, Roles, Sharing | **KEEP → group "Workspace"** | Workspace-scoped, correctly placed. |
| Encryption | **KEEP → group "Compliance"** | Ambient trust surface; read-only key status. |
| About (version/git-sha/built-at/storage backend) | **REMOVE → Admin › System** | Duplicated build info; ops detail belongs in Admin. |
| Audit (stub → Activity) | **REMOVE** | Dead link with no independent content; the sidebar item covers it. |
| Retention | **RELOCATE → Admin** | Admin-level policy masquerading as a user preference. |
| Workspace config | **RELOCATE → Admin** | Admin decision, not a personal setting. |

Groups: **Personal** (Account, Display, Notifications, Tokens) · **Workspace** (Members, Roles,
Sharing) · **Compliance** (Encryption). One primary action per section; hairline/glass re-skin only.

### 5.3 Admin — 9 → 6 tiles

| Tile | Verdict | Rationale |
|---|---|---|
| System (build info + metrics + storage adapter) | **KEEP, consolidate** | Absorbs Settings→About and the Storage-adapter + Sessions subsections. |
| Users | **KEEP** | Core admin. |
| Encryption & keys | **KEEP** | Read-only KEK/DEK/rotation state, never key material. |
| Integrity | **KEEP** | Chain verification at hub scope. |
| Retention & legal hold | **KEEP, absorb Settings→Retention** | The correct, admin-gated home for policy. |
| Audit log | **KEEP (= Activity), absorb Recent sign-ins** | One audit surface; sign-ins are audit rows. |
| Storage adapter (standalone) | **CONSOLIDATE → System** | A subsection, not a top-level tile. |
| Sessions (standalone) | **CONSOLIDATE → System metrics** | Ops metric, not a tile. |
| Recent sign-ins (standalone) | **CONSOLIDATE → Audit log** | Belongs in the audit stream. |

---

## 6. Prioritized change list (re-skin + relayout)

Legend: **⚠ DOM/testid-sensitive** — changes structure/roles/data-testid that e2e may assert; handle
by keeping the existing testid on the surviving element (or migrating the selector in the same PR).
Glass finish (materials/motion) is a pure re-skin and must **not** touch testids (glass §9).

### P0 — high-impact layout wins

1. **Remove top-bar search + recents popover; promote ⌘K to Spotlight.** ⚠ Removes the search input
   node + `RecentSearchesPopover`; migrate any `search`/`recents` testids/e2e onto the ⌘K flow.
2. **Delete Kind, Lock, Encryption columns; make Version conditional.** ⚠ Changes `VAULT_GRID` and the
   table header/row DOM; audit column-count / cell testids and update in the same PR.
3. **Collapse DetailsPanel to one compliance card** (remove Info + People/Share tabs; History →
   summary + link). ⚠ Removes tab triggers/panels; keep the panel root testid, retire tab testids.
4. **Delete coming-soon nav items** (Recent/Starred/Shared) and the no-op **Star** button. ⚠ Minor —
   remove any e2e that asserts these exist.
5. **Establish `/document/{id}/history` as the sole version-history home**; ensure holds banner + verify
   + restore + export live there. (Route already exists; verify feature-completeness.)

### P1 — placement + consolidation

6. **Relocate view + density toggles to Settings › Display.** ⚠ Removes top-bar `ViewToggle` /
   `DensityToggle`; move their testids to the new Settings controls.
7. **Remove the top-bar help button; keep ⌘K "Keyboard shortcuts" + `?` key.** ⚠ Remove help-button
   testid; keep HelpModal + its shortcut path.
8. **Preview modal: sidebar off by default, add the proof one-liner, remove ×, move Share to
   ShareDialog.** ⚠ Changes modal DOM; keep the modal + Open-button testids, retire Star/Close.
9. **Gate Notifications bell to `home` and pause polling off-home.** Behavioral; low DOM risk.
10. **Expand ⌘K with context-aware compliance actions** (Verify ⌘⇧V, Sign, Hold, Export) + Quick
    Create (⌘N / ⌘⇧N / Upload). Additive.
11. **Settings 12 → 8 and Admin 9 → 6** re-scope: move Retention + Workspace config to Admin; build
    info to Admin › System; remove Settings→Audit stub. ⚠ Removes/moves section nodes; update Settings
    nav testids.
12. **Rename Activity → Audit (or add subtitle); add tamper ↔ audit cross-links.** ⚠ Label + link only.

### P2 — glass finish + polish (pure re-skin, no testid changes)

13. **Ambient ground + `.glass` material system** in `tokens.css` (glass §2–3).
14. **Shell (sidebar/top bar) + Spotlight** to `--mat-thin` / `--mat-thick` glass with spring motion.
15. **Panels (compliance card / history / audit) + dialogs / menus / toasts** to `--blur-overlay` glass.
16. **Vault table + badges + empty states** finish pass: glass header, near-solid rows, hover lift,
    amber-wash selection, frosted status pills (verified green tint, tamper `--accent-glow`).
17. **Encryption footer chip** → glowing glass pill; **≥ 11px** legible type (AA on fallback solid).
18. **Light-theme frosted variant + `prefers-reduced-transparency` / `prefers-reduced-motion` QA.**
19. **Responsive:** < 800px single-click → fullscreen; hide file-presence stack, collapse Share to
    kebab in the editor header.
20. **Mirror WorkspaceSwitcher / AvatarStack decisions** once multi-workspace + co-edit MVP status is
    confirmed (remove or redesign as a compact dropdown / row-level presence).

### Acceptance

Reads as a polished, Apple-grade records vault: fewer resting controls, one search surface, one
version-history home, one compliance card, one place encryption is stated. Density unchanged; all
compliance affordances present and correctly placed; AA on fallback solids; reduced-transparency /
reduced-motion fully functional; e2e green (testids preserved or migrated within the same PR).

---

*End of v3. Layers on `ui-system.md` (identity/density/compliance) and `ui-system-glass.md`
(material/motion/Spotlight); where this doc changes **placement**, it wins.*
