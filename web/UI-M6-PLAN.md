# UI M6 — Glass finish + IA relayout (implementation plan)

Applies the **APPROVED** redesign to the Doc-Hub SPA (`web/`): a simultaneous
**relayout** (`docs/design/ui-redesign-v3.md`) **and glass finish**
(`docs/design/ui-system-glass.md`). This plan is the build contract; it encodes
the review amendments that override the spec, and the e2e migrations that keep
the suite 100% green.

> Identity is preserved: density scale (32px rows), ink/paper + amber palette,
> compliance affordances, documents-only scope. WCAG AA is measured on the glass
> **fallback solid** (`--mat-thick` / `#1b1b21`), never the translucent value.
> `prefers-reduced-transparency` and `prefers-reduced-motion` stay fully
> functional (solids, no motion, no loss of meaning).

## Review amendments (these OVERRIDE the source spec)

1. **KEEP the top-bar search field AND ⌘K Spotlight.** The spec cut the search
   input; the amendment reinstates it. The top bar keeps: **logo · search field ·
   notifications · encryption/key glyph · account**. Only the view toggle, density
   toggle, and help button are removed from the top bar. `RecentSearchesPopover`
   stays wired to the search field.
   - Consequence: `search-perf.spec.ts` and the `files.spec.ts` "global search"
     test — both keyed on `getByPlaceholder("Search documents and folders")` —
     **stay green with NO migration**. Do not touch the search placeholder text.
2. **FULL CUT everywhere else, as specced:**
   - Vault table **8 → 5 columns**: drop Kind, Lock, Encryption; Version conditional.
   - DetailsPanel **3 tabs → 1 compliance card** (proof line + "View history →").
   - Delete dead surfaces: coming-soon nav (Recent/Starred/Shared), Settings→Audit
     stub, top-bar help button.
   - **One canonical version-history home**: `/document/{id}/history`; the panel
     summarizes + links to it.
   - Preview: remove the no-op **Star** button and the redundant **×** (Esc +
     scrim-click already close).

---

## 1. FOUNDATION — glass tokens, materials, ambient ground, spring motion

**File:** `src/styles/tokens.css` (single owner: bucket A). Additive only — do NOT
edit or remove existing density/palette/radius tokens; glass layers on top.

### 1.1 Add material + motion tokens to `:root`

```css
:root {
  /* Ambient ground (dark-first hero). */
  --ground: #0e0e12;
  --ground-aurora-1: rgba(183,121,31,0.10);   /* amber bloom */
  --ground-aurora-2: rgba(70,70,90,0.14);     /* cool bloom  */

  /* Material hierarchy (translucency + blur). */
  --mat-ultrathin: rgba(28,28,34,0.44);
  --mat-thin:      rgba(28,28,34,0.60);
  --mat-regular:   rgba(24,24,30,0.72);
  --mat-thick:     rgba(20,20,26,0.86);
  --blur-mat: 20px;      /* regular panels        */
  --blur-chrome: 30px;   /* sidebar / top bar     */
  --blur-overlay: 40px;  /* palette / dialogs     */
  --saturate: 180%;

  /* Edge light + ambient shadow. */
  --edge-hi: inset 0 1px 0 rgba(255,255,255,0.08);
  --edge-lo: inset 0 -1px 0 rgba(0,0,0,0.30);
  --shadow-float:   0 8px 30px rgba(0,0,0,0.38), 0 2px 8px rgba(0,0,0,0.28);
  --shadow-overlay: 0 24px 70px rgba(0,0,0,0.50);
  --hairline-glass: 1px solid rgba(255,255,255,0.10);

  /* Amber, in glass (tamper / attention / focus). */
  --accent-glow: 0 0 0 1px rgba(183,121,31,0.5), 0 0 16px rgba(183,121,31,0.35);

  /* Motion (supersede the old fast/linear set). */
  --ease-spring: linear(0, 0.35 7%, 0.9 18%, 1.05 28%, 1 38%, 1);
  --dur-micro: 120ms;
  --dur-panel: 220ms;
  --dur-overlay: 260ms;
}
```

### 1.2 `.glass` mixin + fallbacks

```css
.glass {
  background: var(--mat-regular);
  backdrop-filter: blur(var(--blur-mat)) saturate(var(--saturate));
  -webkit-backdrop-filter: blur(var(--blur-mat)) saturate(var(--saturate));
  border: var(--hairline-glass);
  box-shadow: var(--edge-hi), var(--shadow-float);
  border-radius: var(--radius-lg);
}
@supports not (backdrop-filter: blur(1px)) {
  .glass { background: #1b1b21; }
}
@media (prefers-reduced-transparency: reduce) {
  .glass { background: #1b1b21; backdrop-filter: none; -webkit-backdrop-filter: none; }
}
```

Provide modifier helpers used by surfaces: `.glass--thin`, `.glass--thick`,
`.glass--overlay` (override `background`/blur var per depth-map §4 of glass doc).
Each carries the same `@supports`/reduced-transparency fallback to a solid.

### 1.3 Immersive ambient ground

```css
body { background: var(--ground); position: relative; }
body::before {
  content: ''; position: fixed; inset: 0; z-index: -1; pointer-events: none;
  background:
    radial-gradient(circle at 30% 50%, var(--ground-aurora-1) 0%, transparent 50%),
    radial-gradient(circle at 70% 80%, var(--ground-aurora-2) 0%, transparent 60%);
  animation: drift 30s ease-in-out infinite; will-change: transform;
}
@keyframes drift { 0%,100% { transform: translate(0,0);} 50% { transform: translate(8px,-12px);} }
@media (prefers-reduced-motion: reduce) { body::before { animation: none; } }
```

### 1.4 Light-theme frosted variant

```css
[data-theme='light'] {
  --ground: #eceae3;
  --mat-ultrathin: rgba(255,255,255,0.44);
  --mat-thin:      rgba(255,255,255,0.55);
  --mat-regular:   rgba(255,255,255,0.68);
  --mat-thick:     rgba(255,255,255,0.82);
  --edge-hi: inset 0 1px 0 rgba(255,255,255,0.6);
  /* soften shadows/glow for light; amber text stays --amber-700 for AA */
}
```

### 1.5 Compliance gates (verify at the end of every bucket)

- AA contrast for all body text **measured on the fallback solid** (`#1b1b21`
  dark, `--mat-thick` light); amber text uses `--amber-700`.
- Every glass surface reduces to a solid under `@supports not` **and**
  `prefers-reduced-transparency: reduce`.
- Every spring/scale/opacity animation is wrapped in
  `@media (prefers-reduced-motion: reduce)` → no transform/opacity animation.
- Never encode state in translucency alone — always icon + label.

---

## 2. Per-surface edits

### Bucket A — Shell + Spotlight

#### `src/components/TopBar.tsx`
- **KEEP** (amendment): logo, the `role="search"` input + `RecentSearchesPopover`,
  `NotificationsBell`, the encryption/key status glyph, the account dropdown.
  **Do not change** the search placeholder `"Search documents and folders"`.
- **REMOVE**: view toggle (Grid/List), density toggle (Comfortable/Compact), and
  the help IconButton (the `?` shortcut path via `Shell.tsx` + `HelpModal` remains).
  Migrate the view/density state to Settings › Display (bucket C owns Settings; A
  exposes/keeps the shared store hook so C can bind the controls — coordinate the
  store location, no file overlap).
- **Material**: wrap the bar in `--mat-thin` glass, `--blur-chrome`, `--edge-hi`;
  height stays 48px. `--shadow-sm`/float appears only when content scrolls under.
- Bell badge: small glass badge with amber glow when unread.

#### `src/components/Sidebar.tsx`
- **REMOVE**: the `comingSoon: true` LIBRARY entries (Recent, Starred, Shared).
- **REMOVE** `AvatarStack` render (gate off until co-edit MVP). `WorkspaceSwitcher`:
  remove unless multi-workspace is live (single-workspace install → remove).
- **KEEP + restyle** `EncryptionFooterChip` → glowing glass pill (`--mat-thin` +
  `--accent-glow` on the lock icon); keep it ≥11px, AA on fallback solid.
- **Material**: sidebar `--mat-thin` glass, `--blur-chrome`.

#### `src/components/NotificationsBell.tsx`
- Add prop `currentNav` (Shell passes `nav`). Render `null` unless
  `currentNav === "home"`; **pause polling** off-home (effect keyed on
  `currentNav`). Preserve the `Notifications` button name + dropdown testable text
  (`help-and-bell.spec.ts` bell tests must stay green).

#### `src/pages/Shell.tsx`
- Pass `nav` into `<NotificationsBell currentNav={nav} />`.
- Keep `?`-key → HelpModal wiring intact (help-and-bell `?` test stays green).
- Apply ambient ground / glass wrappers at the shell root.

#### `src/components/CommandPalette.tsx` (Spotlight)
- **Material**: `--mat-thick` glass, `--blur-overlay`, `--shadow-overlay`,
  `--radius-xl`, spring entrance (`--ease-spring`, ~180ms), dimmed+blurred scrim;
  reduced-motion → instant.
- **Add sections**: context-aware **ACTIONS** (only when a doc/version is
  selected: Verify chain ⌘⇧V, Export bundle, Place legal hold, Sign this version),
  **QUICK CREATE** (New document ⌘N, New folder ⌘⇧N, Upload). Keep GO TO / search
  results (DOCUMENTS · FOLDERS · NOTES). Filter coming-soon (Recent/Starred/Shared)
  out of GO TO.
- Selected row: amber-wash + `--accent-glow` leading icon.

### Bucket B — Documents (vault columns + preview/editor)

#### `src/components/ds/SkeletonRow.tsx`
- Change `VAULT_GRID` from
  `"24px minmax(0,1fr) 96px 56px 96px 44px 96px 32px"` (8 cols) to
  `"24px minmax(0,1fr) 56px 120px 96px 80px 32px"` (**6 tracks**:
  checkbox · name · version\* · status · modified · size · kebab).
- Update `BAR_WIDTHS` to match the new column count/widths.

> Note: the amendment says "8→5 columns" (Kind/Lock/Encryption dropped, Version
> conditional). The MAP's grid template materializes as 6 tracks because Status
> and Size are explicit columns; Version is the conditional one. Ship the 6-track
> template above; keep Version cell empty (not absent) when not
> compliance-significant so the grid stays aligned.

#### `src/components/FileRow.tsx`
- **REMOVE**: kind label, lock icon span, encryption chip span.
- **ADD**: Status column (icon + label — `shield-check`/`shield-alert`,
  `gavel` hold, `badge-check` signed; verified soft-green tint, tamper
  `--accent-glow`; never color-only). Add Size column (formatted bytes).
- **Version** cell conditional: render `v{n}` only when compliance-significant
  (`hold | retention_due | requires_signature | versions > 1`); else render an
  empty placeholder cell.
- **Material**: header row `--mat-thin` glass; data rows near-solid `--mat-thick`
  (legibility). Hover = soft lift (shadow + 1px), not a color swap. Selected =
  amber-wash + `--edge-hi` + 2px left amber rule. 32px row height preserved.

#### `src/components/PreviewModal.tsx`
- **REMOVE**: Star button (import + render) and the close **×** button (keep Esc +
  scrim-click). Remove the always-on 320px right Details sidebar and its
  `gridTemplateColumns: "1fr 320px"`.
- **KEEP**: `data-testid="preview-expand"` (Expand → fullscreen), Download,
  Prev/Next arrows, the primary Open action.
- **ADD**: single-column preview stage; title + **proof one-liner**
  (`🔒 Encrypted · v{n} · ✓ Verified`) under the title; a **Details** toggle that
  mounts `DetailsPanel` on demand (drawer/overlay), and Share routed to the
  existing `ShareDialog`. Viewport < 800px: single-click routes straight to
  fullscreen.
- **Material**: modal `--blur-overlay` glass, `--shadow-overlay`, spring entrance;
  on-demand Details panel `--mat-regular` glass.

#### `src/pages/FileFullscreen.tsx` (editor host)
- Keep header chrome testids: `file-fullscreen`, `file-fullscreen-share`,
  `file-fullscreen-back`, `file-fullscreen-title`, `file-fullscreen-title-input`,
  `file-fullscreen-details`, `file-fullscreen-details-drawer`, and `More actions`.
  The Details drawer now renders the **1-card** DetailsPanel (bucket C content) —
  no tabs. Coordinate only via DetailsPanel's public surface; B does not edit
  DetailsPanel.

### Bucket C — Compliance (DetailsPanel → card, version-home, dead surfaces, settings/admin)

#### `src/components/DetailsPanel.tsx`
- **REMOVE**: `TABS` array, `TabButton`, `InfoTab`, `PeopleTab`, active-tab state,
  and all tab testids (`details-tab-info`, `-info-panel`, `-people`,
  `-people-panel`, `-people-error`, `-people-loading`, `details-people-create-share`,
  `details-tab-history`, `-history-panel`, `details-share-row-*`).
- **KEEP** `data-testid="details-panel"` on the root.
- **REPLACE** render with one un-tabbed **compliance card**
  (`data-testid="details-compliance-card"`):
  - Line 1: `🔒 Encrypted at rest · AES-256-GCM`
  - Line 2: `⛓ Version v{n} · {prior} prior · ✓ Verified`
  - Actions: **View full history →** (`navigate('/document/{id}/history')`) and
    **Share** (opens `ShareDialog`).
- **Material**: card `--mat-regular` glass, `--edge-hi`, `--shadow-float`,
  `--radius-lg`.

#### `src/pages/VersionHistoryPage.tsx` (canonical `/document/{id}/history`)
- Ensure feature-complete: holds banner, tamper alarm (persistent `--accent-glow`
  banner, `role="alert"`, icon+label), **Verify chain** (⌘⇧V), Restore per version,
  Export. Cross-link: tamper alarm → the Activity event that broke the chain;
  Activity version rows → back here. Keep the `Verify chain` button name (asserted
  by the migrated history test).
- **Material**: glass nodes + subtle spine, day/section headers `--mat-thin`.

#### Dead-surface deletions
- `src/components/Sidebar.tsx` coming-soon entries (shared boundary with bucket A —
  **A owns Sidebar edits**; C only removes the corresponding Settings→Audit stub and
  any nav references it owns). To avoid file overlap, **all Sidebar edits belong to
  bucket A**; C flags the coming-soon removal to A. See §4 ownership note.
- `src/pages/settings/*`: remove the Audit stub section.

#### Settings / Admin consolidation
- `src/pages/Settings.tsx` + `src/pages/settings/*`: 12 → 8 sections in three
  groups — **Personal** (Account, **Display** [theme + relocated view + density],
  Notifications, Tokens/Sessions), **Workspace** (Members, Roles, Sharing),
  **Compliance** (Encryption, read-only key status). Remove About (→ Admin ›
  System), Audit stub, Retention (→ Admin), Workspace config (→ Admin). Bind the
  Display view/density controls to the shared store exposed by bucket A; give them
  stable testids.
- `src/pages/Admin.tsx` + `src/pages/admin/*`: 9 → 6 tiles in order **System ·
  Users · Encryption & keys · Integrity · Retention & legal hold · Audit log**.
  System absorbs build info + storage adapter + sessions; Retention & legal hold
  absorbs Settings→Retention; Audit log absorbs Recent sign-ins. Keep the `Admin`
  heading + `Healthy` + `Active sessions` text (asserted by `help-and-bell.spec.ts`
  Admin test — stays green).
- `src/pages/Activity.tsx`: keep the full-page audit log; optional `Audit` subtitle;
  add the tamper↔audit cross-links. Keep the `Activity` heading (asserted by
  help-and-bell bell tests).
- **Material** (Activity/Settings/Admin): day-group/section headers `--mat-thin`;
  rows near-solid `--mat-thick`; tamper banner `--accent-glow` `role="alert"`.

---

## 3. E2E migration list

All specs live in `tests/e2e/`. Migrate assertions ONLY for intentional
removals/relocations; do not weaken assertions for surfaces that still exist.

| File | Test / line | Action | New assertion |
|---|---|---|---|
| `help-and-bell.spec.ts` | `"the help button in the top bar also opens the modal"` (L16–19) | **REMOVE test** | Help button gone; the `?`-key test (L9–14) stays and covers HelpModal. |
| `help-and-bell.spec.ts` | bell/Admin tests (L21–42) | **KEEP unchanged** | `Notifications`, `View all activity`, `Activity`/`Admin`/`Healthy`/`Active sessions` all still exist. |
| `_chrome-gate.spec.ts` | `"FileFullscreen Details pill opens drawer with same panel"` (L152–165) | **EDIT L161** | Replace `getByTestId("details-tab-info")` with `getByTestId("details-compliance-card")`. Keep drawer open/Esc-close assertions. |
| `_chrome-gate.spec.ts` | `"PreviewModal Details panel mounts all 3 tabs"` (L167–176) | **REPLACE test** | New `"PreviewModal Details card shows compliance summary + links"`: open file → `details-panel` visible → `details-compliance-card` visible → buttons `/View full history/i` and `/Share/i` visible. |
| `_chrome-gate.spec.ts` | `"Details People tab → empty state + Create share CTA"` (L178–185) | **REMOVE test** | People tab removed; sharing is via `ShareDialog`. |
| `_chrome-gate.spec.ts` | `"Details History tab → hash-chained version-history surface"` (L187–200) | **REPLACE test** | New `"Details compliance card links to full version history"`: open file → `details-panel` → click `/View full history/i` → `expect(page).toHaveURL(/\/document\/.*\/history/)` → `Verify chain` button visible. |
| `_chrome-gate.spec.ts` | preview/editor chrome tests (Expand, Share+kebab, rename, SDK chrome, fallbacks) | **KEEP unchanged** | `preview-expand`, `file-fullscreen*` testids all preserved. |
| `search-perf.spec.ts` | `getByPlaceholder("Search documents and folders")` (L32) | **KEEP unchanged** | Amendment keeps the top-bar search field; do not touch placeholder. |
| `files.spec.ts` | `"global search narrows the result set"` (L77–90) | **KEEP unchanged** | Search field kept; `Search results` / `My Drive` headings unchanged. |
| `files.spec.ts` | nav/upload/rename/sort/select tests | **KEEP unchanged** | Role/text selectors survive the reshuffle; columns removed aren't asserted here. |
| `sign-in.spec.ts`, `sdk-integration.spec.ts`, `_iframe-verify.spec.ts`, `_visual-final.spec.ts` | — | **KEEP unchanged** | Higher-level selectors; no removed testids. |

Testid summary: **keep** `details-panel`, `preview-expand`, `file-fullscreen*`,
search placeholder, `Notifications`/`Activity`/`Admin` text. **Add**
`details-compliance-card` + Display-section control testids. **Remove** all
`details-tab-*`, `details-people-*`, `details-share-row-*`, and the top-bar help
button.

---

## 4. Three DISJOINT implementer buckets

Each bucket owns a non-overlapping file set **and migrates its own e2e tests**.
Shared concern: view/density store — bucket A defines/keeps the hook + its home
file; bucket C imports it read/write. Agree the hook path up front; neither edits
the other's files.

### Bucket A — Shell + Spotlight
- **Source**: `src/components/TopBar.tsx`, `src/components/Sidebar.tsx`,
  `src/components/NotificationsBell.tsx`, `src/pages/Shell.tsx`,
  `src/components/CommandPalette.tsx`, (+ `RecentSearchesPopover.tsx` if touched).
  Owns ALL Sidebar edits incl. the coming-soon removal.
- **Foundation dependency**: consumes tokens from `tokens.css`. To avoid a write
  conflict, **bucket A also owns `src/styles/tokens.css`** (§1) since the shell is
  the first glass consumer.
- **E2E it owns**: `help-and-bell.spec.ts` (remove help-button test; keep the rest).

### Bucket B — Documents (vault + preview/editor)
- **Source**: `src/components/ds/SkeletonRow.tsx`, `src/components/FileRow.tsx`,
  `src/components/PreviewModal.tsx`, `src/pages/FileFullscreen.tsx`
  (header chrome only; renders DetailsPanel but does not edit it).
- **E2E it owns**: `_chrome-gate.spec.ts` preview/editor rows that touch
  `preview-expand` and `file-fullscreen*` (keep green; edit L161 only if it lands
  in B's scope — otherwise C owns the DetailsPanel-content edits below).

### Bucket C — Compliance (details card + version-home + dead surfaces + settings/admin)
- **Source**: `src/components/DetailsPanel.tsx`,
  `src/pages/VersionHistoryPage.tsx`, `src/pages/Settings.tsx` +
  `src/pages/settings/*`, `src/pages/Admin.tsx` + `src/pages/admin/*`,
  `src/pages/Activity.tsx`.
- **E2E it owns**: `_chrome-gate.spec.ts` DetailsPanel tab migrations (L152–200:
  edit L161 to `details-compliance-card`, replace the 3-tab test, remove People,
  replace History→route). Because both B and C need `_chrome-gate.spec.ts`, **C
  owns the DetailsPanel-related edits (L152–200) and the L161 change; B owns the
  Expand/header rows (L79–150)** — the file is edited by whoever lands first, then
  rebased; assign `_chrome-gate.spec.ts` primarily to **C** to keep the
  DetailsPanel migration atomic, with B handing C its (unchanged) expectations.

> Ownership resolution for the two shared files:
> - `tokens.css` → **A** (foundation ships with the shell).
> - `_chrome-gate.spec.ts` → **C** (owns the DetailsPanel test block); B's rows are
>   unchanged, so C's edit is purely additive to that file. B does not modify it.

---

## 5. Acceptance

- e2e suite 100% green: help-button test removed; DetailsPanel 3-tab tests →
  1-card/route tests; search + preview/editor/bell/admin tests unchanged.
- Top bar: logo · search · bell (home-only) · key glyph · account — ambient glass.
- Vault: 6-track grid (Version conditional), status + size columns, near-solid rows.
- DetailsPanel: single compliance card linking to `/document/{id}/history`.
- Dead surfaces gone (coming-soon nav, Star, ×, Settings→Audit, top-bar help).
- Settings 12→8, Admin 9→6, one version-history home.
- AA on fallback solids; reduced-transparency/motion fully functional; density,
  ink/paper+amber identity, and documents-only scope unchanged.
