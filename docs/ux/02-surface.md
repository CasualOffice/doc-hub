# 02 — UI Surface Spec

The visual layer beneath the flows in [`01-flows.md`](./01-flows.md). For each surface: ASCII layout, the components and tokens that build it, every state it can be in, the keyboard model, and the motion. Still no pixel mockups — that's Figma or the implementation, this is the bridge.

Calibration: everything draws from the token set + libraries in [`../research/04-polish-principles.md`](../research/04-polish-principles.md). When a spacing/radius/colour value appears, it cites the token (`--space-3`, `--radius-md`, `--bg-elevated`). Components reference Radix Primitives, shadcn/ui patterns, cmdk, vaul, sonner, and Lucide.

Cross-cutting:
- All spacing snaps to the 4/8 grid.
- Concentric corners: container `--radius-lg` (12 px) with `--space-3` (12 px) inner padding → inner element `--radius-xs` (4 px) is the rule of thumb.
- Hairline borders use `--border-default` (`rgba(0,0,0,0.08)` light, `rgba(255,255,255,0.08)` dark).
- Focus uses `:focus-visible` only; the global focus ring token is `--focus-ring`.
- Every surface has a dark-mode variant; the colour tokens swap under `[data-theme="dark"]` (auto from `prefers-color-scheme`, overridable via setting).

## Contents

1. App shell (window, top bar, sidebar, main pane, footer hooks)
2. Sidebar
3. Top bar
4. Breadcrumbs + sort header
5. File list view
6. Icon / grid view (deferred surface; spec sketched)
7. Empty states
8. Selection bar
9. Command palette
10. Modals
11. Toasts
12. Drop zones + inline upload row
13. Sign-in card
14. Recipient share page
15. Editor-handoff badge + Open button

---

## 1 — App shell

```
┌──────────────────────────────────────────────────────────────────────┐
│ ☁ Casual Drive                                              ⌘K   👤  │  ← top bar (48px)
├────────┬─────────────────────────────────────────────────────────────┤
│        │ Home › Reports › Q2                                          │  ← breadcrumbs (40px)
│ Home   │ ┌─ Name ─────────── Modified ─── Size ─── Type ──┐  ▭ ▢ ⬚   │  ← sort header (36px)
│ Recent │ ├──────────────────────────────────────────────────────────┤ │
│ Starred│ │ 📄 Budget Q2.xlsx     2 hrs ago   42 KB   Spreadsheet    │ │  ← main pane
│ Shared │ │ 📁 Drafts             yesterday    —      Folder         │ │
│ Trash  │ │ 🖼  hero.png          last week    1.2 MB Image          │ │
│        │ │ ...                                                       │ │
│ ──     │ │                                                           │ │
│ + New  │ └──────────────────────────────────────────────────────────┘ │
│        │                                          [Preparing zip 23%] │  ← footer pill (32px, optional)
└────────┴─────────────────────────────────────────────────────────────┘
   240px              auto
```

**Layout.**

- Three columns: sidebar (240 px expanded / 52 px collapsed), main pane (fluid), no right panel in v0.
- Top bar: 48 px tall, full width, sticky, `--bg-default` with hairline bottom border.
- Sidebar: full height, `--bg-canvas` with `backdrop-filter: saturate(180%) blur(20px)` on top of a 70% opaque fill (vibrancy). Hairline right border.
- Main pane: `--bg-default`, no border on top (sort header carries its own).
- Footer pill: floating bottom-center inset 24 px, only when a job is in progress (zip prep, large upload aggregate). `--bg-elevated`, `--shadow-md`, `--radius-full`.

**Sizing & spacing.**

| Surface | Token |
|---|---|
| Top bar height | 48 px |
| Sidebar width (expanded) | 240 px |
| Sidebar width (collapsed) | 52 px |
| Main pane padding | `--space-6` (24 px) all sides; top is consumed by breadcrumbs |
| Footer pill height | 32 px |
| Section gap inside sidebar | `--space-2` (8 px) |

**Responsive.**

- ≥ 1024 px: layout above.
- 720 – 1023 px: sidebar starts collapsed; click-to-expand overlays the main pane (`--shadow-lg`, dismiss on outside click).
- < 720 px: top bar gains a hamburger icon; sidebar becomes a vaul drawer slid from left. File list compacts to a stacked vertical card layout (no columns). **Out of v0 scope; design hook only.**

**Motion.**

- Sidebar expand/collapse: 200 ms `--ease-out`, width + label-opacity together.
- Theme flip: 250 ms colour interpolation on shell tokens. Sun/moon glyph rotates 180° in the same window.

---

## 2 — Sidebar

```
┌────────────────┐
│ ☁ Casual Drive │  ← brand row, 48px, double-click → /
├────────────────┤
│                │
│ 🏠 Home        │  ← active: --bg-selected, 2px accent stripe on left
│ 🕒 Recent      │
│ ⭐ Starred     │
│ 👥 Shared      │
│ 🗑  Trash  (3) │  ← badge for trashed item count
│                │
│ ──────────     │
│                │
│ ＋ New ▾       │  ← split button: folder, upload
│                │
└────────────────┘
│                │
│ 👤 A           │  ← bottom-anchored avatar menu trigger
└────────────────┘
```

**Sections (top to bottom).**

1. **Brand row** (48 px). Lucide `cloud` glyph (20 px, `--accent`) + wordmark in `--font-display`, `--text-md`, `--weight-semibold`. Click → navigate to `/`. Same height as top bar so the two align.
2. **Primary nav.** Five items: Home, Recent, Starred, Shared, Trash. Each row is 32 px tall, `--space-3` left padding, Lucide glyph (16 px) + label (`--text-sm`, `--weight-medium`). Trash item shows `(N)` badge in `--fg-muted` `--text-xs` if non-empty.
3. **Separator.** 1 px `--border-default`, full width, no margin.
4. **+ New ▾.** Split-button. Default action: Upload (matches the toolbar). Dropdown: Folder · File upload · Folder upload. Same row metrics as primary nav.
5. **(Future) Pinned folders.** Out of v0; design hook is the same row component.
6. **Avatar pinned to bottom.** 40 px tall, `--space-3` padding. Single-letter monogram in a 28 px circle (`--bg-subtle` background, `--fg-default` text). Tooltip on hover: full admin name. Click → dropdown menu (Radix DropdownMenu) with: **Account**, **Settings**, separator, **Sign out** (with chord chip `⇧⌘Q`).

**Row states.**

| State | Visual |
|---|---|
| Default | transparent bg, `--fg-default` label, `--fg-subtle` glyph |
| Hover | `--bg-hover` background |
| Active (selected) | `--bg-selected` background, 2 px `--accent` left-edge stripe, `--accent` glyph |
| Focus-visible | `--focus-ring` shadow (4 px outset, accent at 60%) |
| Drop-target (during drag) | `--accent-muted` background, dashed 1 px `--accent` border |

**Collapsed state.**

- Width 52 px. Labels removed; only glyphs. Tooltip on hover (Radix Tooltip, 250 ms delay).
- Brand row keeps glyph only.
- Avatar collapses to just the monogram circle.
- Toggled via icon-button at the top-right of the sidebar (Lucide `panel-left-close` / `panel-left-open`). Shortcut: `⌘\`.

**Keyboard.** `⌘\` toggle collapse. `⌘1`–`⌘5` jump to nav items 1–5.

---

## 3 — Top bar

```
┌──────────────────────────────────────────────────────────────────────┐
│ ☁ Casual Drive          │ ┌─ 🔍 Search files or run a command... ─┐│ ⇧⌘K  👤 │
└──────────────────────────────────────────────────────────────────────┘
```

**Layout.**

- 48 px tall, full width, sticky.
- Left: brand mirror (only visible when sidebar is collapsed, to avoid duplication).
- Center: search trigger — a faux-input that *opens the command palette* on click or `⌘K`. Not a real input. Placeholder: **"Search files or run a command…"**. Width: clamp(320 px, 40vw, 560 px). Lucide `search` 16 px on left. `⌘K` chord chip muted on right.
- Right: avatar (28 px monogram, same as sidebar bottom — duplicated only when sidebar is collapsed, to keep the menu reachable).

**States of the search trigger.**

- Default: `--bg-subtle` background, hairline border, `--fg-muted` placeholder.
- Hover: `--bg-hover` background, `--border-strong` border.
- Focus / palette-open: `--bg-default` background, `--accent` 1 px border, `--shadow-sm`.

**Polish.**

- The "press ⌘K" chip is muted text rendered as `font-variant-numeric: tabular-nums`. Don't make it a clickable button — it's documentation.
- When the palette opens, the trigger keeps its focus appearance so the eye chain is unbroken.

---

## 4 — Breadcrumbs + sort header

```
Home › Reports › Q2                                                  [List] [Grid]
─────────────────────────────────────────────────────────────────────────────────
Name ▲           Modified           Size           Type
```

**Breadcrumbs.**

- 40 px tall band, `--bg-default`, no border above, hairline below.
- Items: `--text-sm`, `--weight-medium`. Current item in `--fg-default`, prior items in `--fg-muted`. Separator: a 12 px `›` glyph in `--fg-subtle`, padded 6 px each side.
- Long paths truncate the middle: `Home › Reports › … › Q2`. The `…` is a Radix DropdownMenu trigger; click reveals the intermediate levels as a list.
- View-mode toggle on the right: 2-button segmented control (List / Grid), uses Radix ToggleGroup. Currently active button has `--bg-selected`, accent text.

**Sort header.**

- 36 px tall band, sticky to top of scroll container (under breadcrumbs).
- Columns: Name (flex), Modified (160 px), Size (96 px right-aligned), Type (120 px). Hidden columns: configurable later; v0 ships these four only.
- Header cells use `--text-xs`, `--weight-medium`, `--fg-muted`, with arrow glyph (▲/▼) appearing on the active sort column in `--fg-default`.
- Click toggles sort direction; shift-click adds a secondary sort (advanced; v0 supports primary only).
- The whole row has `--border-default` hairline bottom.

**Polish.**

- Resizable columns: hairline gutter between headers becomes `--accent` on hover with `col-resize` cursor.
- Tabular numerals on Size column (and all numeric columns). Right-aligned.

---

## 5 — File list view

```
│ 📄  Budget Q2.xlsx              2 hrs ago     42 KB    Spreadsheet  │  ← 32px row, default
│ 📁  Drafts                       yesterday      —       Folder       │  ← hover: --bg-hover
│ ▶ 📁 Q1 (open)                  3 days ago     —       Folder       │  ← editor session badge
│ 🖼   hero.png                   last week     1.2 MB   Image        │
│ ⌫   Removed File.pdf             3 days ago    -       PDF          │  ← uploading/deleted ghost
```

**Row.**

- 32 px tall (compact), `--space-3` (12 px) left padding, `--space-4` (16 px) right padding.
- Layout: type icon (16 px Lucide) → name (`--text-sm`, `--weight-medium`, truncate with `overflow-wrap: anywhere`) → spacer → modified date (`--fg-muted`, `--text-sm`) → size (`--font-variant-numeric: tabular-nums`, right-aligned) → type label (`--fg-muted`).
- Type icons: pick from a tiny shared table — folder, spreadsheet, document, presentation, image, video, audio, pdf, archive, code, generic.

**Row states.**

| State | Visual |
|---|---|
| Default | transparent |
| Hover | `--bg-hover` |
| Focused (keyboard) | `--bg-hover`, focus ring on the row (not inset — outset so it doesn't shift content) |
| Selected | `--bg-selected`, 2 px `--accent` left-edge stripe |
| Selected + focused | both, focus ring on top |
| Editor session active | small `Open` badge (`--bg-accent-muted`, `--accent`, `--text-xs`, `--weight-medium`) inline after the name |
| Uploading | ghost row: dimmed to 60% opacity, thin determinate progress bar across the bottom 2 px of the row in `--accent`, type icon overlaid with Lucide `upload-cloud` glyph |
| Upload failed | row tints `--danger-muted`, tooltip with the failure reason on hover |
| Drag origin | row dims to 40% opacity; cursor carries a card representation |
| Drop target (folder rows only) | `--accent-muted` background; the folder icon glow `--accent` |

**Inline edit (rename).**

- Row enters rename mode: name cell swaps to an input that exactly matches the row's typography. The extension stays inline as `--fg-muted` non-editable text.
- Validation tint: `--danger` 1 px border + helper line below the row (compact).
- Esc cancels, Enter commits. Tab commits + jumps to rename on the next row.

**Keyboard.**

- ↑↓ move focus. `Cmd-A` select all. `Enter` open. `F2` rename. `Backspace`/`Delete` trash. `Esc` clear selection. Letter-key jumps (sticky 1 s). `Home`/`End` first/last.

**Motion.**

- Reflow on insert/delete: animate by FLIP (First-Last-Invert-Play) over 200 ms `--ease-out`. Sonner-style.
- Selection toggle: no animation — selection should feel instant.
- Uploading row: progress bar updates frame-locked to real progress events; no synthetic interpolation.

**Virtualisation.**

- Use `@tanstack/react-virtual` for any list > 100 rows. Row height is fixed (32 px) so virtualisation is trivial.

---

## 6 — Icon / grid view (deferred surface; spec sketched)

```
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│          │  │          │  │          │  │          │
│   📁     │  │   📄     │  │   🖼     │  │   📑     │
│          │  │          │  │          │  │          │
│ Drafts   │  │Budget Q2 │  │hero.png  │  │Slides    │
│          │  │  .xlsx   │  │          │  │          │
└──────────┘  └──────────┘  └──────────┘  └──────────┘
```

- 120 × 120 px tiles by default, `--space-4` gap.
- Big type icon (48 px Lucide) above a 2-line truncated name in `--text-sm`.
- Same states as the list row (hover, focus, selected, drag origin, drop target, uploading).
- Rubber-band lasso selection.
- Resizing handle in the toolbar gives 96 / 120 / 160 / 200 px tile sizes (4 stops, not a slider).
- v0 ships List view default; Grid view as a toggle. Gallery + Column views deferred.

---

## 7 — Empty states

Pattern: centred column, ~480 px max-width, vertical flow.

```
                              ╭───────────╮
                              │   📂     │   ← 56px Lucide glyph, --fg-subtle
                              ╰───────────╯
                          Your Drive is empty.       ← --text-xl, --weight-semibold
                       Drop files anywhere, or       ← --text-md, --fg-muted
                            use Upload.
                         ┌──────────────┐
                         │   Upload   U │            ← primary button + chord chip
                         └──────────────┘
```

**Variants.**

| Surface | Title | Subtitle | CTA |
|---|---|---|---|
| Home (first run) | "Your Drive is empty." | "Drop files anywhere, or use Upload." | Upload (primary) |
| Folder (no items) | "This folder is empty." | "Drop files to add." | — |
| Search (no results) | "No files match \"<q>\"." | — | Clear search (text link) |
| Trash (empty) | "Trash is empty." | "Files you delete will appear here." | — |
| Shared (none) | "Nothing shared with you." | — | — |
| Recent (none) | "Nothing recent yet." | "Files you open will appear here." | — |

**Polish.**

- Glyph: 56 × 56 px, `--fg-subtle`, never animated.
- Fade in at 200 ms after shell mount.
- Never include a tutorial overlay or annotation arrows.

---

## 8 — Selection bar

```
                          ┌───────────────────────────────────────────────┐
                          │ 3 selected   ⬇ Download  →  Move…  🔗 Share…  │
                          │              🗑 Trash                     ×    │
                          └───────────────────────────────────────────────┘
```

**Layout.**

- Bottom-centered, inset 24 px from the bottom edge of the main pane.
- vaul-based drawer (not a toast — persistent until cleared).
- Width: hugs content with min 480 px / max 720 px. Single row on desktop; wraps on narrow viewports.
- Background: `--bg-elevated` at 80% opacity + `backdrop-filter: saturate(180%) blur(20px)`. Vibrancy.
- Border: hairline `--border-default`. Radius: `--radius-xl` (16 px).
- Shadow: `--shadow-lg`.

**Contents (left to right).**

1. Count chip: **"N selected"** in `--text-sm`, `--weight-medium`.
2. Action chips: each is a Radix DropdownMenu-style row item with Lucide glyph + label. Hover `--bg-hover`. Spacing `--space-2`.
3. Compact divider (vertical 1 px hairline) between non-destructive and destructive actions.
4. Trash chip (`--danger` text + glyph, no background).
5. Spacer.
6. Clear button (×). `Esc` chord chip on hover tooltip.

**Motion.**

- Enter: slide up 200 ms `--ease-out` + fade. Spring `{stiffness: 400, damping: 30}`.
- Exit: slide down 150 ms `--ease-in` + fade.
- Action chips have 80 ms hover transitions (background only).

**Keyboard.** `Esc` dismiss. Actions inherit their global chords (`Cmd-D` download, `Cmd-Shift-M` move, etc.).

---

## 9 — Command palette

```
┌──────────────────────────────────────────────────────────────┐
│ 🔍 Search files or run a command…                       Esc │  ← input row, 56px
├──────────────────────────────────────────────────────────────┤
│ FILES                                                        │  ← section header, --fg-subtle, --text-xs, --weight-medium, uppercase tracking
│   📄 Budget Q2.xlsx                  Home › Reports › Q2     │  ← 36px row
│   📁 Drafts                          Home                    │
│ ─────                                                        │
│ COMMANDS                                                     │
│   ＋ New folder                                       ⌘⇧N    │
│   ⬆ Upload                                            U     │
│   🌗 Toggle theme                                     ⌘⇧L    │
│   ⏏ Sign out                                          ⇧⌘Q    │
└──────────────────────────────────────────────────────────────┘
```

**Layout.**

- 600 px wide, centred horizontally, top-aligned 120 px below viewport top (so cursor doesn't need to chase up).
- Radius `--radius-xl`, shadow `--shadow-xl`, hairline `--border-default`.
- Background: `--bg-elevated`, optional backdrop-filter for the dim under it (40% opacity overlay on canvas).
- Built on cmdk; section headers via `Command.Group`; chord chips via custom right-slot.

**Sections.**

| State | Files | Commands | Empty |
|---|---|---|---|
| Initial (no query) | Recents (up to 5) | All commands (sorted) | — |
| Query typed | Matches (up to 8) | Matches | — |
| No results | — | — | "No files match \"<q>\"." centred at row 4 |
| Loading | Skeleton (4 rows × 36 px) | (commands still shown) | — |

**Row format.**

- 36 px tall, `--space-3` left padding, glyph (16 px) → label (`--text-sm`, `--weight-medium`) → spacer → meta (file path muted, or chord chip).
- Highlighted row: `--bg-selected`, `--accent` glyph.

**Motion.**

- Enter: 150 ms `--ease-out` opacity + 4 px translate-Y.
- Exit: 120 ms `--ease-in` opacity.
- No bounce. Calm.

**Keyboard.** ↑↓ move highlight. Enter select. Esc close. ⌘K from anywhere opens; ⌘K from open palette closes.

---

## 10 — Modals

There are exactly four modals in v0. Each uses Radix Dialog.

### 10.1 Empty Trash confirm

```
┌─────────────────────────────────────────────────────┐
│ Empty Trash?                                        │  ← title
│ This will permanently delete 23 items.              │  ← body
│ You can't undo this.                                │
│                                                     │
│                              [Cancel]  [Empty Trash]│  ← actions, danger primary
└─────────────────────────────────────────────────────┘
```

- 440 px wide, `--radius-xl`, `--shadow-xl`, `--space-6` padding all sides.
- Title: `--text-lg`, `--weight-semibold`.
- Body: `--text-md`, `--fg-muted`.
- Buttons: Cancel ghost, Empty Trash filled `--danger`. Right-aligned with `--space-2` gap.
- Open: spring-up `{stiffness: 300, damping: 28}` from 0.97 scale + fade. Backdrop fades to 40% opacity.

### 10.2 Move to… picker

```
┌─────────────────────────────────────────────────────┐
│ Move 3 items to…                                Esc │
├─────────────────────────────────────────────────────┤
│ 🔍 Search folders                                   │
├─────────────────────────────────────────────────────┤
│ 📁 Home                                             │
│   📁 Reports                                        │
│     📁 Q1                                           │
│     📁 Q2     ← cursor                              │
│     📁 Q3                                           │
│   📁 Drafts                                         │
│ 📁 Shared with me                                   │
└─────────────────────────────────────────────────────┘
│                              [Cancel]  [Move here]  │
```

- 520 px wide × 480 px tall.
- Header search input filters the folder tree live.
- Tree rendered with disclosure chevrons; current path expanded by default.
- Primary action label changes to reflect destination: **"Move to *Q2*"** once a folder is highlighted.
- Up/Down navigates; Right expands a folder; Left collapses; Enter is "Move here".

### 10.3 Share modal

```
┌─────────────────────────────────────────────────────┐
│ Share Budget Q2.xlsx                            ✕   │
├─────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────┐ 📋 │  ← URL + copy
│ │ https://drive.example.org/s/Xa7b…           │     │
│ └─────────────────────────────────────────────┘     │
│ Anyone with this link can view.                     │  ← live caption
│                                                     │
│ Who can…       [ View | View + Download | Edit ]    │
│ Password       ◯ off                                │
│ Expires        ● 7 days from now    [Jun 14, 2026]  │
│                                                     │
│ ─── Existing links ───                              │
│ /s/9Pk2…     View       2 days ago     [Copy] [⋯]   │
└─────────────────────────────────────────────────────┘
```

- 480 px wide, expands taller as content grows.
- URL field is a read-only `<input>` with constant-width font (`--font-mono`), `--bg-subtle` background, hairline border.
- Copy button: Lucide `copy`, tooltip "Copy link" → "Copied" (1 s revert + 200 ms `--bg-selected` flash on the field).
- Segmented control for perms: Radix ToggleGroup.
- Password input: appears with a `--space-2` smooth height transition when toggle on; strength indicator below (4 bars).
- Expires date: Radix Popover with a small day-grid calendar.
- "Existing links" list: separated by `--space-4` and a hairline; rows = 32 px.

### 10.4 Conflict resolver (upload / move)

```
┌─────────────────────────────────────────────────────┐
│ "Budget Q2.xlsx" already exists in Reports.         │
├─────────────────────────────────────────────────────┤
│ ☐ Apply to all 4 conflicts                          │
│                                                     │
│  [Skip]    [Keep both]    [Replace]                 │
└─────────────────────────────────────────────────────┘
```

- 440 px wide. Title is contextual (the conflict).
- Three-action row, equal width; Skip ghost, Keep both ghost, Replace filled `--accent`.
- "Apply to all" checkbox is the only non-action surface — Radix Checkbox.

**Modal cross-cutting.**

- Backdrop: `rgba(0, 0, 0, 0.40)` with `backdrop-filter: blur(2px)`.
- Esc and outside click dismiss (except Empty Trash, which requires explicit Cancel).
- Focus traps inside; first interactive element gets focus on open; on close focus returns to the trigger.

---

## 11 — Toasts

sonner. Top-right corner, stacked, max 3 visible (oldest collapses into "+N more" if more arrive).

**Anatomy.**

```
┌─────────────────────────────────────────────────┐
│ ✓ Moved 3 items to Reports.        [Undo]   ✕   │
└─────────────────────────────────────────────────┘
```

- 360 px wide, `--radius-lg`, `--shadow-md`, hairline border.
- Background: `--bg-elevated`.
- Icon (16 px Lucide) → message (`--text-sm`, `--weight-medium`) → action button (text only, `--accent`) → close (×, `--fg-subtle`).

**Variants.**

| Variant | Glyph | Glyph colour | Use |
|---|---|---|---|
| Success | `check-circle-2` | `--success` | "Uploaded N files.", "Moved to…" |
| Info | `info` | `--info` | "Signed out for security." |
| Warning | `alert-triangle` | `--warning` | "Trash uses 1.2 GB. Empty?" |
| Error | `alert-circle` | `--danger` | "Couldn't move. Try again?" |

**Lifetime.**

- Default: 4 s.
- With undo: 8 s.
- Error: 6 s + auto-dismiss-on-action.
- Hover: pause lifetime; resume on mouse-leave.

**Motion.**

- Enter: slide-in from the right 200 ms `--ease-out` + fade. Spring `{stiffness: 400, damping: 30}`.
- Exit: slide-out 150 ms `--ease-in` + fade.

**Microcopy rules.** Lean Linear-terse. Verb-first. No exclamation marks unless genuinely celebratory (we won't have any).

---

## 12 — Drop zones + inline upload row

### Window-wide drop zone

```
╔══════════════════════════════════════════════════════╗
║                                                      ║
║                                                      ║
║                  ┌──────────────────┐                ║
║                  │   ⬆               │                ║
║                  │ Drop to upload   │                ║
║                  │   to Reports     │                ║
║                  └──────────────────┘                ║
║                                                      ║
║                                                      ║
╚══════════════════════════════════════════════════════╝
```

- Activated by dragging files from OS over the Drive window.
- Whole canvas dims to `--bg-subtle` over 120 ms.
- Centered card: 320 × 160 px, dashed 2 px `--accent-muted` border, `--radius-xl`, Lucide `upload-cloud` 32 px, `--text-md` `--weight-medium` caption.
- Caption substitutes the destination folder name.

### Folder-row drop target

- Row tints `--accent-muted` background.
- Folder icon glows `--accent`.
- After 700 ms hover: spring-loaded expand (navigates into folder, animates the breadcrumb).

### Inline upload row

(same as §5 "Uploading" row state — the row appears in the list as a ghost with a determinate progress bar across its bottom 2 px in `--accent`).

---

## 13 — Sign-in card

```
                  ╭─────────────────────────────╮
                  │            ☁                │
                  │      Casual Drive            │
                  │   Sign in to continue.       │
                  │                              │
                  │   ┌────────────────────┐     │
                  │   │ Password           │     │
                  │   └────────────────────┘     │
                  │                              │
                  │   [        Sign in       ]   │
                  │                              │
                  ╰─────────────────────────────╯
```

- 360 px wide, centred (vertical + horizontal).
- `--radius-xl`, `--shadow-md`, hairline border, `--bg-default`.
- Lucide `cloud` 28 px in `--accent` at top.
- Brand wordmark `--text-xl`, `--weight-semibold`.
- Subhead `--text-md`, `--fg-muted`.
- Single password input full-width, `--radius-md`, hairline border, focus → `--focus-ring`.
- Primary button full-width, `--radius-md`, `--accent` fill.
- Caps-lock helper text appears below the input in `--fg-muted` `--text-xs` when relevant.
- Background: solid `--bg-canvas`; no marketing imagery. (Operator can swap in a hero background via config later.)
- Shake-on-error: 8 px lateral, 250 ms, eased. 1 cycle only.

---

## 14 — Recipient share page

```
                  ╭──────────────────────────────╮
                  │                              │
                  │       📄                     │  ← type icon 56px
                  │                              │
                  │   Budget Q2.xlsx             │  ← --text-xl --weight-semibold
                  │   42 KB · Spreadsheet        │  ← --text-md --fg-muted
                  │                              │
                  │   Shared by Sachin           │  ← --text-sm --fg-muted
                  │                              │
                  │   [   Open in editor    ]    │  ← primary, accent
                  │   [   Download         ]     │  ← secondary, ghost
                  │                              │
                  ╰──────────────────────────────╯
                          Powered by Casual Drive
```

- Deliberately stripped of Drive chrome: no sidebar, no top bar, no avatar.
- 440 px wide centred card, vertical layout, `--radius-xl`, `--shadow-md`.
- Page background: `--bg-canvas` (light) or `--bg-canvas-dark` — auto by `prefers-color-scheme`.
- "Powered by Casual Drive" footer link: 12 px below the card, `--fg-subtle`, `--text-xs`. Operator can disable via config (`DRIVE_RECIPIENT_FOOTER=false`).

**Password gate variant.**

```
                  ╭──────────────────────────────╮
                  │       🔒                     │
                  │   Enter password to open     │
                  │   ┌──────────────────────┐   │
                  │   │ Password             │   │
                  │   └──────────────────────┘   │
                  │   [     Continue       ]     │
                  ╰──────────────────────────────╯
```

**Preview-with-content variant.** For images/PDFs/text, the card grows to fit the preview; primary action remains.

---

## 15 — Editor-handoff badge + Open button

### Split button

```
[ Open ] [ ▾ ]
```

- Single Radix DropdownMenu split-button.
- Primary action: **Open** (new tab, edit-mode).
- Dropdown:
  - **"Open in this tab"** with `Shift-Enter` chord chip.
  - **"Open as read-only"** with `Cmd-Enter` chord chip.
  - separator.
  - **"Copy link"** (creates a share-link with current default perms).

### Active session badge

```
📄  Budget Q2.xlsx  [Open]               2 hrs ago    42 KB    Spreadsheet
```

- Badge `Open`: `--bg-accent-muted` background, `--accent` text, `--text-xs`, `--weight-medium`, `--radius-xs`, `--space-1` (4 px) padding y, `--space-2` (8 px) padding x.
- Hover: tooltip **"Editing — open since 14:32"**.
- The badge appears within ~500 ms of an editor session establishing a WOPI lock; disappears within ~30 s of the lock release.

---

## Component → token cheat sheet

| Component | Surface tokens | Notes |
|---|---|---|
| Top bar | `--bg-default`, `--border-default` | sticky, 48 px |
| Sidebar | `--bg-canvas` + 70% opacity + `backdrop-filter` | vibrancy |
| Main pane | `--bg-default` | scroll container |
| Row | hover `--bg-hover`, sel `--bg-selected`, focus `--focus-ring` | 32 px |
| Button (primary) | `--accent` bg, `--fg-onAccent`, `--radius-md` | `--space-3` × `--space-2` padding |
| Button (ghost) | transparent, `--border-default`, hover `--bg-hover` | same shape |
| Button (danger) | `--danger` bg, `--fg-onAccent` | confirm contexts |
| Input | `--bg-default`, `--border-default`, focus `--focus-ring` | `--radius-md` |
| Toast | `--bg-elevated`, `--border-default`, `--shadow-md`, `--radius-lg` | 360 px |
| Modal | `--bg-elevated`, `--shadow-xl`, `--radius-xl` | spring open |
| Popover / dropdown | `--bg-elevated`, `--border-default`, `--shadow-lg`, `--radius-lg` | tween open |
| Chord chip | `--bg-subtle`, `--fg-muted`, `--text-xs`, `--font-mono`, `--radius-xs` | inline tag |

## States checklist (per surface)

Each surface must specify all of these or explicitly waive them:

- [ ] Default
- [ ] Hover
- [ ] Focus-visible (keyboard)
- [ ] Active / pressed
- [ ] Selected (where applicable)
- [ ] Loading (skeleton or progress)
- [ ] Empty (where applicable)
- [ ] Error
- [ ] Disabled (rare in Drive — usually we hide instead)
- [ ] Drop target (where applicable)

The 16 flows × the above states form the test matrix Drive's component library must cover before any flow is "done".

## What this doc deliberately doesn't cover (deferred)

- **Mobile / narrow viewport** beyond a noted hook.
- **Gallery & column views** in the file list.
- **Quick Look-style preview pane**.
- **Settings surface** — full inventory of toggles and forms.
- **Onboarding tutorial** — there isn't one in v0 by design.
- **Marketing-quality first-launch art** — none in v0; the empty state is the welcome.
- **Visual mockups** (Figma / image renders). The next step is implementation against this spec, not more docs.
