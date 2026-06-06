# 04 — Premium File-Manager / Data-Table List Surface Patterns (2026)

**Audience:** the frontend engineer building Casual Drive's main file pane.
**Purpose:** capture the 2026 state of the art for "list of files / list of records" surfaces (Linear, Vercel, Stripe, Figma, Dropbox, Drive, Notion, Finder, Arc), distill the cross-cutting rules, then **replace surface §5 (file list) and §8 (selection bar) in [`../ux/02-surface.md`](../ux/02-surface.md)** with an implementable spec.
**Constraint:** WebSearch only; WebFetch denied on product hosts. Numbers from design-system mirrors / write-ups are flagged `[unverified]` and should be confirmed against the live UI.

---

## TL;DR

- **Row metric:** 32 px × 13 px Inter / 500 is the Linear-derived SaaS benchmark of 2026 ([Linear DS mirror](https://styles.refero.design/style/90ce5883-bb24-4466-93f7-801cd617b0d1), [getdesign.md Linear](https://getdesign.md/linear.app/design-md)). Vercel's May 27 2026 deployments redesign ratified the same pull ("denser layout") ([Vercel changelog](https://vercel.com/changelog/redesigned-deployments-list)). Dropbox 44 px and Drive 48–56 px are the "consumery" tells.
- **Type:** body 13 / metadata 12 / header 11 uppercase, Inter, `tabular-nums` on every numeric column ([uiprep](https://www.uiprep.com/blog/the-ultimate-guide-to-designing-data-tables)).
- **Hover:** background tint only, no border, no revealed icons. Skip Drive's hover-checkbox and Notion's hover-OPEN; both add noise.
- **Selection:** file-manager convention (click = select, Cmd-click = add, Shift-click = range, double-click = open). Not Linear's `X`-toggle (engineer-specific) ([Linear select-issues](https://linear.app/docs/select-issues)).
- **Keyboard:** ↑↓ + Enter + Cmd-A + Esc + letter-jump + F2/Enter + Backspace/Delete. Matches ARIA Grid pattern ([ARIA APG Grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)).
- **Virtualization:** TanStack Virtual, threshold >100 rows, `useFlushSync: false` for React-19 ([TanStack Virtual](https://tanstack.com/virtual/latest), [Borstch](https://borstch.com/blog/development/list-virtualization-in-react-with-tanstack-virtual)).
- **Drag-drop:** Atlassian's **Pragmatic drag-and-drop** — production-proven (Trello/Jira/Confluence), external adapter handles OS-file drops ([Pragmatic DnD](https://github.com/atlassian/pragmatic-drag-and-drop), [PkgPulse 2026](https://www.pkgpulse.com/guides/dnd-kit-vs-react-beautiful-dnd-vs-pragmatic-drag-drop-2026)).
- **Density:** ship one (32 px). Sonoma System Settings is the cautionary tale ([Lapcat](https://lapcatsoftware.com/articles/SystemSettings.html)).
- **Motion:** Motion `layout` for FLIP on insert/move/delete ([Motion docs](https://motion.dev/docs/react-layout-animations)); AutoAnimate the one-liner ([AutoAnimate](https://awesome-react.dev/library/auto-animate)). **No animation on selection.**
- **Selection bar:** floating bottom-center pill, persistent until cleared. Table stakes now ([Eleken](https://www.eleken.co/blog-posts/bulk-actions-ux), [PatternFly](https://www.patternfly.org/patterns/bulk-selection/)).
- Drive spec at bottom replaces §5 + §8 of `02-surface.md`.

---

## Reference list surfaces

**1. Linear — issues list (gold standard).** ~32 px rows default, 28 px in compact `[unverified]`; Inter Variable 510/590, body ~13 px `[unverified]` ([Linear DS mirror](https://styles.refero.design/style/90ce5883-bb24-4466-93f7-801cd617b0d1), [Made Good Designs on Inter](https://madegooddesigns.com/inter-font/)). User-reorderable columns via Display menu ([Linear display options](https://linear.app/docs/display-options)). Hover background only, no border ([UI refresh Mar 2026](https://linear.app/changelog/2026-03-12-ui-refresh)). **Focus and selection are separate layers** — arrows move a "cursor," `X` toggles selection, `Shift-↑↓` extends, `Cmd-A` all, `Shift-click`/`Cmd-click` work too ([Linear select-issues](https://linear.app/docs/select-issues), [issue selection changelog](https://linear.app/changelog/issue-selection)). Right-click = same menu as Cmd-K on selection. Sort/group/sub-group live in a Display menu, not in headers. Custom virtualization present `[unverified]`. Bulk: floating contextual bar ([Storylane](https://www.storylane.io/tutorials/how-to-bulk-edit-issues-in-linear)). One density.

**2. Linear — inline edit.** Title/description: click-to-edit, autosave ([2022 changelog](https://linear.app/changelog/2022-06-09-inline-editing)). Properties (status/assignee/priority/labels): **chord-driven pickers** scoped to focused row — `S`/`A`/`P`/`L` open a Cmd-K-shaped picker. Drive adopts the same shape for rename (F2/Enter) and Move-to (⌘⇧M).

**3. Vercel — projects / deployments list.** May 27 2026 explicitly went **denser**, grouped environments by status, made branches/commits scannable ([Vercel deployments redesign](https://vercel.com/changelog/redesigned-deployments-list)). Feb 26 2026 rolled out collapsible sidebar: "projects function as filters so you can switch between team and project versions in one click" ([dashboard rollout](https://vercel.com/changelog/dashboard-navigation-redesign-rollout)). Row: project name (semibold), framework icon, last-deploy time, environment badges; hover reveals row-end action menu; explicit `⋯` overflow replaces right-click as the visible affordance. Row height `[unverified]`, visually ~32 px.

**4. Stripe — payments table.** Published Table primitives expose no row-height tokens ([Stripe Apps Table](https://docs.stripe.com/stripe-apps/components/table)). Row click opens a side panel; double-click unused — sidesteps file-manager open/select ambiguity. Tabular-nums + right-aligned numerics are the universal standard ([uiprep](https://www.uiprep.com/blog/the-ultimate-guide-to-designing-data-tables), [Carbon DS](https://carbondesignsystem.com/components/data-table/usage/)). Hover: light tint; selected: stronger tint + left-edge stripe `[unverified]`. Bulk: top-of-table action bar.

**5. Figma — file browser.** List and grid views ("Show as grid" toggle) ([file browser guide](https://help.figma.com/hc/en-us/articles/14381406380183-Guide-to-the-file-browser), [forum grid hover](https://forum.figma.com/suggest-a-feature-11/file-type-icons-in-recent-files-grid-gallery-view-17317)). Grid tiles render 16:9 from 1920×1080 source ([thumbnail guide](https://help.figma.com/hc/en-us/articles/360038511413-Set-custom-thumbnails-for-files), [design a thumbnail](https://help.figma.com/hc/en-us/articles/23510169950871-Design-a-file-thumbnail)). Grid-tile hover: type icon top-left, context-menu trigger top-right. Drafts is a private workspace with identical row visuals ([drafts updates](https://help.figma.com/hc/en-us/articles/18409526530967-Updates-to-how-drafts-work)). Empty: centered illustration + heading + "Create new" CTA. Spring-loaded folder expansion observed, undocumented `[unverified]`.

**6. Dropbox web.** ~44 px row `[unverified]` to fit avatar + thumbnail. Hover reveals inline share/⋯ icons at row end. Right-click on web + OS shell extensions ([CBackup](https://www.cbackup.com/articles/dropbox-right-click-menu-missing.html)). Multi-select: hover-checkbox + top-of-list action bar. **Lesson: looser row is a consumer choice — wrong precedent for Linear-grade.**

**7. Google Drive.** Row ~48–56 px `[unverified]` for owner avatar + hover thumbnail. May 2024 introduced **hovercard preview** ([Workspace Updates](https://workspaceupdates.googleblog.com/2024/05/preview-files-in-google-drive-with-hovercards.html), [9to5Google](https://9to5google.com/2024/05/16/google-drive-hovercard/)) — polarizing; users publicly call the hover-checkbox "annoying" ([community thread](https://support.google.com/drive/thread/205464794/when-we-hover-mouse-on-a-file-folder-it-shows-a-selection-option-as-a-checkmark-which-is-annoying)). Right-click duplicates toolbar verbatim — the "feature-of-features" tell. **Does well:** shared-with badges, owner attribution, type-aware thumbnails. **Does poorly:** density (too loose), hover noise, menu duplication.

**8. Notion — database table.** Every cell editable on click ([Notion tables](https://www.notion.com/help/tables)). Hover reveals **OPEN button** in col 1, **⋮⋮ drag handle** left, **checkbox** at row start ([Medium](https://medium.com/@VaughanVanDyk/notion-databases-10-things-i-needed-to-learn-52873eb2618b)). Keyboard: `Return` cell-below, `Shift-Return` newline, `Cmd-D`/`Cmd-R` fill-down/right ([shortcuts](https://www.notion.com/help/keyboard-shortcuts)). Sort/filter in header dropdown. Column resize via edge drag. **Lesson: don't reveal too many controls on hover.** Notion's three (drag + OPEN + checkbox) is busy. Drive shows **nothing** on hover except a tint.

**9. macOS Finder — list view.** Row ~22 px small / ~38 px medium `[unverified]` — same density philosophy as Linear, twenty years earlier. Column widths not persistable as defaults in list view ([Apple Discussions](https://discussions.apple.com/thread/8304069)) — Drive must persist them to surpass Finder. macOS Tahoe 26.1 added "Resize Columns To Fit File Names" auto-fit in column view ([MacMost](https://macmost.com/resize-columns-to-fit-filenames.html)). Rename: `Return` enters rename, extension *not* selected ([Apple Discussions](https://discussions.apple.com/thread/255445067)). Right-click is the primary command surface; ~12 default verbs.

**10. Arc — sidebar Tabs.** ~36 px rows with `padding: 0 12px` flexbox ([ArcWTF Firefox port CSS](https://github.com/KiKaraage/ArcWTF/blob/main/README.md)). Subtle hover bg, no border. Three visual tiers (pinned / everyday / favorite) distinguished by spacing and size, not color ([Blake Crosley](https://blakecrosley.com/guides/design/arc), [LogRocket UX](https://blog.logrocket.com/ux-design/ux-analysis-arc-opera-edge/)). **Lesson: when 30+ rows stack in a scrollable column, the row component is the brand. Get it right; everything else is decoration.**

---

## Synthesis

**Row height.** 32 px = 2026 benchmark. 28 px is floor (Apple HIG min-tappable, [Nadcab](https://www.nadcab.com/blog/apple-human-interface-guidelines-explained)). 36–40 px = "comfortable" (avatar rows). >44 px = consumer. **Drive: 32 px.**

**Type.** Body 13 / Inter 500. Metadata 12 muted. Header 11 uppercase + `letter-spacing: 0.04em`. Tabular-nums on every numeric column.

**Hover.** Background tint only. Linear/Vercel/Notion/Stripe: tint only. Dropbox/Drive: tint + revealed icons (consumer tell). **Drive: tint only**; inline icons on focus or selection, never hover.

**Selection.** Two models: Linear (focus/selection separate, `X` toggles, keyboard-optimized) vs file-manager (click = select, Cmd-click = add, Shift-click = range). **Drive picks file-manager.** Users come in with Finder/Explorer muscle memory.

**Keyboard.** Matches the ARIA Grid pattern ([W3 ARIA APG Grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)): ↑↓ move focus+selection · Home/End first/last · PgUp/PgDn page · Enter open (folder navigates, file opens editor/downloads) · Space Quick Look (deferred) · Cmd-A select all · Esc clear · Cmd-click toggle · Shift-click / Shift-↑↓ range · letter-key jump (sticky 1 s) · F2 or Enter rename (slow double-click rename, fast double-click open) · Backspace/Delete trash · Cmd-Shift-Space context menu at focus.

**Virtualization.** TanStack Virtual is the React-19 default ([TanStack Virtual](https://tanstack.com/virtual/latest), [LogRocket](https://blog.logrocket.com/speed-up-long-lists-tanstack-virtual/), [Borstch](https://borstch.com/blog/development/list-virtualization-in-react-with-tanstack-virtual)). Threshold >100 rows, `useFlushSync: false` for React-19, `estimateSize: 32`, `overscan: 5`. Selection `Set<id>` outside the virtualizer. Alternatives: react-window (smaller API), react-virtualized (aging), [react-arborist](https://github.com/jameskerr/react-arborist) (trees w/ virtualization + drag + multi-select + inline edit out of the box, [LogRocket](https://blog.logrocket.com/using-react-arborist-create-tree-components/)) — sidebar candidate, not for the flat main pane.

**Drag-drop.** **Pragmatic drag-and-drop** powers Trello/Jira/Confluence; external adapter handles OS-file cleanly ([Pragmatic DnD](https://github.com/atlassian/pragmatic-drag-and-drop), [docs](https://atlassian.design/components/pragmatic-drag-and-drop)). DnDKit is modular (12 KB core, [Zoer](https://zoer.ai/posts/zoer/best-react-drag-drop-libraries-comparison)) but bolts the OS-file case onto raw HTML5. React Aria `useDragAndDrop` has the best a11y + native `FileDropItem` ([RAC DnD](https://react-spectrum.adobe.com/react-aria/dnd.html)) — pick if RAC is otherwise in the stack. **Drive picks Pragmatic** unless the component-library brief lands on RAC.

**Empty state.** Centered in table viewport, not whole pane. Symbol → title → optional subtitle → optional CTA ([Eleken](https://www.eleken.co/blog-posts/empty-state-ux), [Carbon DS](https://carbondesignsystem.com/patterns/empty-states-pattern/), [NN/g](https://www.nngroup.com/articles/empty-state-interface-design/)).

**Density.** Ship one. Linear/Vercel/Stripe/Notion all do.

**Motion.** Motion `layout` for FLIP on insert/move/delete ([Motion](https://motion.dev/docs/react-layout-animations)); AutoAnimate the one-liner ([AutoAnimate](https://awesome-react.dev/library/auto-animate)). No animation on selection (sub-10 ms response). Spring on drag preview. `prefers-reduced-motion` → 0–50 ms crossfade ([MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion)).

**Skeleton.** 8 rows at exact row footprint, 1.2 s `linear-gradient` shimmer, `background-attachment: fixed` keeps rows in sync ([Mat Simon](https://www.matsimon.dev/blog/simple-skeleton-loaders)).

---

## Drive surface spec — main file pane (replaces §5 of `02-surface.md`)

Tokens reference `04-polish-principles.md` §"Starter Token Set".

### Layout (top of pane, downward)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ [ + New ▾ ]   [ ⬆ Upload  U ]                                 [List] [Grid]  │  toolbar 44 px
├──────────────────────────────────────────────────────────────────────────────┤
│ Home › Reports › Q2                                                           │  breadcrumbs 32 px
├──────────────────────────────────────────────────────────────────────────────┤
│ NAME ▲                                MODIFIED        SIZE     KIND          │  sort header 32 px, sticky
├──────────────────────────────────────────────────────────────────────────────┤
│ 📁  Drafts                            yesterday         —     Folder         │  row 32 px
│ 📁  Q1                                3 days ago        —     Folder         │
│ 📄  Budget Q2.xlsx                    2 hrs ago     42 KB     Spreadsheet    │
│ 🖼   hero.png                          last week    1.2 MB    Image          │
│▌📄  Notes.md      [Open]              10 min ago     8 KB     Markdown       │  editor session, left stripe
│ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │  skeleton
└──────────────────────────────────────────────────────────────────────────────┘
                                            (selection bar floats at bottom — §8 below)
```

### Pane-top toolbar

44 px, padding `--space-3` × `--space-6`, `--bg-default`, hairline bottom, sticky.

- **New ▾**: ghost split-button + chevron, 13/500, `--radius-md`. Dropdown: New folder (⌘⇧N) · Upload file (U) · Upload folder (⌘⇧U).
- **Upload**: primary fill (`--accent` / `--fg-onAccent`), `upload-cloud` 16, chord chip `U` muted right.
- **View toggle**: Radix ToggleGroup, two 28 px squares (`list`, `grid-2x2`); active = `--bg-selected` + `--accent` icon; persisted in IndexedDB.

### Breadcrumbs band

32 px (drop from current 40 to match row metrics), 13 px / 500, current segment `--fg-default`, others `--fg-muted`, `›` separator 12 px `--fg-subtle`. Long paths collapse to middle ellipsis dropdown (per current spec).

### Sort header

32 px, `--bg-default`, hairline bottom only, sticky under breadcrumbs. Type 11 px / 500 / `--fg-muted` / `letter-spacing: 0.04em` / uppercase. Active column: `--fg-default` + 12 px ▲/▼ in `--accent`. Click toggles asc → desc → clear (back to default name-asc). Resize handle between header cells on hover (`col-resize` + 1 px `--accent` line).

### Columns

Name (flex min 240, left, 13/500) · Modified (144 px, left, 13/400 muted, tabular-nums) · Size (96 px, right, 13/400 muted, tabular-nums) · Kind (128 px, left, 13/400 muted). All sortable. Widths persist per user in IndexedDB.

### Row tokens

**32 px** fixed. Padding `--space-3` left / `--space-4` right. Transparent (no zebra; Linear/Vercel/Notion skip it). Bottom grid line: 1 px `--border-default` at 50% opacity (hairline, not divider). Type icon: 16 px Lucide; folder tints `--accent` when selected/focused. Name: 13/500/`--fg-default`, ellipsis truncate, `overflow-wrap: anywhere`. Modified: relative ("2 hrs ago", "yesterday", "3 May"), muted, tabular-nums. Size: binary ("42 KB", "—" for folders), muted, right-aligned, tabular-nums. Kind: one-word ("Folder", "Image", "Spreadsheet", "PDF"), muted.

### Row state matrix

- **Default:** transparent. **Hover:** `--bg-hover`, no border, cursor `default`.
- **Focused (kb):** `--bg-hover` + outer `--focus-ring`, offset (no content shift).
- **Selected:** `--bg-selected` + 2 px `--accent` left-edge stripe. **Selected+hover:** +4% toward accent. **Selected+focused:** stack focus ring on top.
- **Editor session:** inline `[Open]` chip (`--bg-accent-muted`, `--accent` text, 11/500, `--radius-xs`, padding 4 × 8) after name; tooltip "Editing — open since HH:MM".
- **Uploading ghost:** 60% opacity; icon overlaid with `upload-cloud`; 2 px determinate `--accent` bar at row bottom, real bytes.
- **Upload failed:** `--danger-muted` tint; icon → `alert-circle` `--danger`; tooltip with reason.
- **Drag origin:** 40% opacity; cursor carries card.
- **Drop target (folder row only):** `--accent-muted` bg, folder glows `--accent`, 2 px ring; spring-loaded expand after 700 ms.
- **Disabled (e.g. trash on editor-locked):** 8 px lateral shake, 200 ms, 1 cycle.

### Inline rename

Trigger F2, Enter on focused row (slow double-click = rename, fast = open, Finder convention), right-click → Rename. Name cell becomes input matching row typography; extension in `--fg-muted` next to input, not editable without explicit click ([Apple Discussions](https://discussions.apple.com/thread/255445067)). Auto-select basename only. Border `--border-strong` → `--danger` on invalid + helper line. Enter commits, Esc cancels, Tab commits + advances. Optimistic; 409 reverts with 8 px shake (200 ms), stays in edit, helper "Already a file with that name."

### Multi-select

Click = select single (replaces prior). Cmd-click = toggle. Shift-click = range from anchor. Cmd-A = all in folder. Esc = clear. Shift-↑↓ = extend. Lasso: list view only when drag begins in whitespace; grid defaults to lasso. State in `Set<file_id>` outside virtualizer; Cmd-A on 10k rows is fast.

### Drag-drop

Library: **Pragmatic drag-and-drop** ([repo](https://github.com/atlassian/pragmatic-drag-and-drop)) for in-app row → folder; external adapter for OS-file → window.

- Whole row draggable, no visible handle (Finder/Drive/Dropbox convention). 4 px start threshold.
- Cursor preview: 32 px floating card at 95% opacity; multi-select shows "(N)" stack badge.
- Drop target (folder row, sidebar folder, breadcrumb segment): bg → `--accent-muted`, folder → `--accent`, 2 px ring.
- Spring-loaded folders: hover > 700 ms → navigate in.
- Drop completion: source fades out 200 ms ease-out; destination flashes `--bg-selected` 1 cycle (200 ms); FLIP on reflow.
- Cancel (Esc): cursor card springs back `{400, 30}`.
- OS-file overlay: canvas dims to `--bg-subtle` (120 ms); centered 320 × 160 card, dashed 2 px `--accent-muted` border, `--radius-xl`, `upload-cloud` 32 px, caption "Drop to upload to *<folder>*"; spring pop-in.
- Invalid drop (folder → itself): cursor `not-allowed`, silently ignored.
- Keyboard fallback: Cmd-Shift-M opens Move-to picker (flow 9).

### Skeleton state

8 rows × 32 px matching layout (16 × 16 icon block, name flex-fill, modified 96, size 64 right-aligned, kind 80). Blocks `--bg-subtle` `--radius-xs`. Shimmer: linear-gradient sweep 1.2 s, `background-attachment: fixed` for sync ([Mat Simon](https://www.matsimon.dev/blog/simple-skeleton-loaders)). `prefers-reduced-motion`: static blocks pulsing opacity 50% ↔ 70% over 1.6 s. Replaces row list only.

### Empty state (centered, in-table)

```
                              ┌───────────┐
                              │    📂     │   56 px Lucide folder-open, --fg-subtle
                              └───────────┘
                          This folder is empty.       20 px / --weight-semibold
                            Drop files to add.         15 px / --fg-muted
```

Root "first run" variant adds `[ Upload  U ]` primary button below. Search no-results: title `No files match "<query>".`, single text-link "Clear search". Fade in 200 ms after skeleton ends. No tutorial overlay; no glyph animation.

### Loading on fetch / pagination

Initial: skeleton above. Paginate-on-scroll: append 4 skeleton rows at list end; replace on arrival. Hover pre-fetch on folder > 100 ms (polish-principle #11) → cached → sub-100 ms navigate render.

### Error state

```
                              ┌───────────┐
                              │    ⚠     │   56 px Lucide alert-triangle, --warning
                              └───────────┘
                       Couldn't load this folder.      20 px / --weight-semibold
                          Check your connection.        15 px / --fg-muted
                            [ Try again ]              ghost button
```

Same vertical-center layout as empty state. Toast does *not* fire concurrently — in-pane error is the surface; toasts are for transients.

### Right-click context menu

Radix ContextMenu at cursor. 220 px wide, `--bg-elevated`, `--shadow-lg`, `--radius-lg`, item rows 28 px / 13 px / 500. Chord chips right-aligned, 11 px `--font-mono`, muted.

- **File selected:** Open (Enter) · Open in new tab (⇧⏎) — Rename (F2) · Move to… (⌘⇧M) · Share… (⌘⇧S) · Copy link — Download (⌘D) — Properties (⌘I) — Move to trash (⌫).
- **Folder selected:** same minus Open-in-new-tab; "Open" navigates in.
- **Multi-select:** drops Rename / Open-in-new-tab / Properties; rest scale to selection; footer shows muted "<N> items".
- **Empty area:** New folder · Upload file · Upload folder · Paste (disabled if clipboard empty) · Sort by ▸ (Name / Modified / Size / Kind).

### Motion summary

- Insert / sort change / undo: Motion `layout` FLIP, 200 ms `--ease-out`, spring `{400, 30}`. Delete: opacity 0 + translateY(-4 px) 200 ms; neighbors FLIP up. Drag-drop reorder: FLIP on drop.
- Hover: `--bg-hover` fade 80 ms. Focus ring: 120 ms opacity fade. **Selection toggle: none — instant.**
- Skeleton shimmer: 1.2 s loop, `background-attachment: fixed`. Drop overlay: dim 120 ms + card pop-in. Drop completion: source fade 200 ms + dest flash 200 ms.
- Rename: instant; only validation-error shake (8 px / 200 ms / 1 cycle).
- `prefers-reduced-motion`: FLIPs → 0–50 ms crossfade; shimmer → opacity pulse.

### Copy strings (final)

Upload button **"Upload"** + chip `U`. New menu **"New folder"** · **"Upload file"** · **"Upload folder"**. Empty root: **"Your Drive is empty."** / **"Drop files anywhere, or use Upload."** / CTA **"Upload"**. Empty folder: **"This folder is empty."** / **"Drop files to add."** / no CTA. Empty search: **"No files match \"<query>\"."** / link **"Clear search"**. Error: **"Couldn't load this folder."** / **"Check your connection."** / **"Try again"**. Drop caption: **"Drop to upload to *<folder>*"**. Editor badge: **"Open"** / tooltip **"Editing — open since HH:MM"**.

### Virtualization

TanStack Virtual ([`@tanstack/react-virtual`](https://tanstack.com/virtual/latest)) with `useFlushSync: false` for React 19 ([Borstch guide](https://borstch.com/blog/development/list-virtualization-in-react-with-tanstack-virtual)). Threshold: `rows.length > 100`. `estimateSize: 32`, `overscan: 5`. Selection `Set<id>` lives outside the virtualizer.

---

## Drive surface spec — selection bar (replaces §8 of `02-surface.md`)

```
                          ┌─────────────────────────────────────────────────────┐
                          │ 3 selected   ⬇ Download   →  Move…   🔗 Share…   │  │
                          │                             🗑 Trash         ⎋ Clear │
                          └─────────────────────────────────────────────────────┘
                                ▲ floating, bottom-center, 24 px inset
```

### Layout

Fixed bottom-center of main pane, 24 px inset. Width hugs content (min 480, max 720). Height 56 px single row (wraps to 2 rows below 640 viewport). Background `--bg-elevated` at 80% opacity + `backdrop-filter: saturate(180%) blur(20px)` (vibrancy). Hairline `--border-default`. `--radius-xl` (16). `--shadow-lg`. `z-index: --z-popover` (1400) — above rows, below modals.

### Contents (left → right)

Count chip `"<N> selected"` (13/500, tabular-nums, plural-aware) → vertical hairline 16 px → action chips (32 px tall, `--space-2` horizontal, Lucide 16 + label 13/500): **Download** (⌘D) · **Move…** (⌘⇧M) · **Share…** (⌘⇧S, only when exactly 1 selected); hover `--bg-hover`, active `--bg-selected` → vertical hairline → **Trash** chip in `--danger` text + glyph, no fill (hover `--bg-hover`, no danger tint on hover) → spacer → **Clear ×** in `--fg-subtle` → `--fg-default` on hover, tooltip "Clear (Esc)". Chord chips are not shown on the bar itself — they live in chip tooltips and in Cmd-K.

### State matrix

- Hidden (0–1 selected): not rendered.
- Enter (≥ 2 selected): slide up 200 ms `--ease-out` + fade, spring `{stiffness: 400, damping: 30}`.
- Exit: slide down 150 ms `--ease-in` + fade.
- Action in progress: active chip gets 2 px `--accent` progress bar across bottom; chip interactable; Esc cancels.
- Action success: chip flashes `--bg-selected` 1 cycle (200 ms); toast carries confirmation.

### Compact mode (< 640 px)

Chips collapse to icon-only with labels in tooltips. Order preserved. Trash stays in danger color.

### Mixed-selection rules

- Folders + files: chips that don't apply (e.g. Share, when v0 folder share isn't supported) are **hidden, not greyed**. Hiding > disabling for "does not apply" ([Linear changelog: issue selection](https://linear.app/changelog/issue-selection)).
- Mixed Download routes to zip-bundle path (flow 16); label stays "Download".

### Keyboard

`Esc` clears + dismisses. `⌘D` / `⌘⇧M` / `⌘⇧S` / `Backspace` / `Delete` fire respective actions globally.

### `prefers-reduced-motion`

Enter/exit become 100 ms opacity fade.

### Copy strings

- Count: **"<N> selected"** (plural-aware: `1 selected`, `2 selected`, …).
- Actions: **Download** · **Move…** · **Share…** · **Trash** · **Clear**.
- Clear tooltip: **"Clear (Esc)"**.
- Trash tooltip: **"Move to trash (Delete)"**.
- Inapplicable chips: silent (not rendered).

---

## States checklist (test matrix)

**File list:** Default · Hover · Focused (keyboard) · Selected (single / multi / range) · Inline rename (open / valid / invalid / conflict) · Uploading ghost · Upload failed · Drag origin · Drop target (folder / sidebar / breadcrumb) · Editor session badge · Skeleton · Empty (root / folder / search) · Error · Right-click menu (file / folder / multi / empty area) · OS-file drop overlay.

**Selection bar:** Hidden · Enter animation · Hover chip · Active chip during in-progress action · Compact viewport · Mixed-selection hiding · Exit animation.

---

## Sources

**Linear:** [board](https://linear.app/docs/board-layout) · [display](https://linear.app/docs/display-options) · [views](https://linear.app/docs/custom-views) · [select](https://linear.app/docs/select-issues) · [create](https://linear.app/docs/creating-issues) · [editor](https://linear.app/docs/editor) · [inline-edit](https://linear.app/changelog/2022-06-09-inline-editing) · [shortcuts](https://linear.app/changelog/2021-03-25-keyboard-shortcuts-help) · [selection](https://linear.app/changelog/issue-selection) · [UI-Mar2026](https://linear.app/changelog/2026-03-12-ui-refresh) · [diffs-May2026](https://linear.app/changelog/2026-05-27-linear-diffs) · [releases-Apr2026](https://linear.app/changelog/2026-04-30-releases) · [shortcuts.design](https://shortcuts.design/tools/toolspage-linear/) · [ShortcutFoo](https://www.shortcutfoo.com/app/dojos/linear-app-mac/cheatsheet) · [Keycombiner](https://keycombiner.com/collections/linear/) · [DS-mirror](https://styles.refero.design/style/90ce5883-bb24-4466-93f7-801cd617b0d1) · [getdesign.md](https://getdesign.md/linear.app/design-md) · [redesign](https://linear.app/now/how-we-redesigned-the-linear-ui) · [Storylane](https://www.storylane.io/tutorials/how-to-bulk-edit-issues-in-linear)

**Vercel:** [redesign-rollout](https://vercel.com/changelog/dashboard-navigation-redesign-rollout) · [deployments-May2026](https://vercel.com/changelog/redesigned-deployments-list) · [changelog](https://vercel.com/changelog) · [new-dashboard](https://vercel.com/try/new-dashboard) · [docs](https://vercel.com/docs/projects/project-dashboard) · [deployments-docs](https://vercel.com/docs/deployments) · [blog](https://vercel.com/blog/dashboard-redesign) · [925studios](https://www.925studios.co/blog/saas-dashboard-design-examples-2026) · [Releasebot](https://releasebot.io/updates/vercel)

**Stripe / data tables:** [Apps-Table](https://docs.stripe.com/stripe-apps/components/table) · [Dashboard](https://docs.stripe.com/dashboard/basics) · [Apps-patterns](https://docs.stripe.com/stripe-apps/patterns) · [SaaSFrame](https://www.saasframe.io/examples/stripe-payments-dashboard) · [uiprep](https://www.uiprep.com/blog/the-ultimate-guide-to-designing-data-tables) · [Carbon-DS](https://carbondesignsystem.com/components/data-table/usage/)

**Figma:** [browser](https://help.figma.com/hc/en-us/articles/14381406380183-Guide-to-the-file-browser) · [drafts](https://help.figma.com/hc/en-us/articles/18409526530967-Updates-to-how-drafts-work) · [drafts-blog](https://www.figma.com/blog/the-power-of-figma-drafts/) · [files](https://help.figma.com/hc/en-us/articles/1500005554982-Guide-to-files-and-projects) · [thumbnails](https://help.figma.com/hc/en-us/articles/360038511413-Set-custom-thumbnails-for-files) · [thumbnail-design](https://help.figma.com/hc/en-us/articles/23510169950871-Design-a-file-thumbnail) · [forum](https://forum.figma.com/suggest-a-feature-11/file-type-icons-in-recent-files-grid-gallery-view-17317)

**Dropbox:** [CSS](https://github.com/dropbox/css-style-guide) · [right-click](https://www.cbackup.com/articles/dropbox-right-click-menu-missing.html) · [context-menu](https://www.tenforums.com/tutorials/158955-how-add-remove-dropbox-context-menu-windows.html) · [PageFlows](https://pageflows.com/web/products/dropbox/) · [NicelyDone](https://nicelydone.club/apps/dropbox)

**Google Drive:** [help](https://support.google.com/drive/answer/2375177) · [hovercard](https://workspaceupdates.googleblog.com/2024/05/preview-files-in-google-drive-with-hovercards.html) · [9to5Google](https://9to5google.com/2024/05/16/google-drive-hovercard/) · [complaint](https://support.google.com/drive/thread/205464794/when-we-hover-mouse-on-a-file-folder-it-shows-a-selection-option-as-a-checkmark-which-is-annoying) · [AODocs](https://support.aodocs.com/hc/en-us/articles/206775853-Switch-from-grid-layout-to-list-layout-in-Google-Drive) · [Gizmodo](https://gizmodo.com/best-cloud-storage/google-drive-alternatives) · [Dragbin](https://www.dragbin.com/reviews/google-drive-review-2026/)

**Notion:** [tables](https://www.notion.com/help/tables) · [databases](https://www.notion.com/help/intro-to-databases) · [shortcuts](https://www.notion.com/help/keyboard-shortcuts) · [dashboards](https://www.notion.com/help/dashboards) · [buttons](https://www.notion.com/help/buttons) · [Medium](https://medium.com/@VaughanVanDyk/notion-databases-10-things-i-needed-to-learn-52873eb2618b) · [VIP](https://www.notion.vip/insights/compare-and-configure-notion-s-database-formats-tables-lists-galleries-boards-and-timelines)

**Finder:** [columns](https://discussions.apple.com/thread/8304069) · [rename](https://discussions.apple.com/thread/255445067) · [resize](https://macmost.com/resize-columns-to-fit-filenames.html) · [MacMost](https://macmost.com/mac-basics-how-to-rename-files.html) · [TidBITS](https://tidbits.com/2018/06/28/macos-hidden-treasures-batch-rename-items-in-the-finder/) · [Automators](https://talk.automators.fm/t/how-to-adjust-column-width-in-apple-mail-or-finder-list-view/17672) · [OSXDaily](https://osxdaily.com/2010/03/25/setting-the-default-column-size-in-mac-os-x-finder-windows/) · [Lapcat](https://lapcatsoftware.com/articles/SystemSettings.html)

**Arc:** [Crosley](https://blakecrosley.com/guides/design/arc) · [Refine](https://refine.dev/blog/arc-browser/) · [LogRocket](https://blog.logrocket.com/ux-design/ux-analysis-arc-opera-edge/) · [ArcWTF](https://github.com/KiKaraage/ArcWTF/blob/main/README.md) · [Wikipedia](https://en.wikipedia.org/wiki/Arc_(web_browser)) · [Medium](https://medium.com/design-bootcamp/arc-browser-rethinking-the-web-through-a-designers-lens-f3922ef2133e)

**Virtualization:** [TanStack-Virtual](https://tanstack.com/virtual/latest) · [GitHub](https://github.com/TanStack/virtual) · [react-docs](https://tanstack.com/virtual/v3/docs/framework/react/react-virtual) · [fixed-ex](https://tanstack.com/virtual/latest/docs/framework/react/examples/fixed) · [Table-virt](https://tanstack.com/table/v8/docs/guide/virtualization) · [virt-rows-ex](https://tanstack.com/table/v8/docs/framework/react/examples/virtualized-rows) · [LogRocket](https://blog.logrocket.com/speed-up-long-lists-tanstack-virtual/) · [Borstch-1](https://borstch.com/blog/development/how-to-virtualize-a-long-list-in-react-using-tanstack-virtual-for-better-performance) · [Borstch-2](https://borstch.com/blog/development/list-virtualization-in-react-with-tanstack-virtual) · [Borstch-3](https://borstch.com/blog/development/practical-guide-to-implementing-fixed-lists-using-tanstack-virtual-in-react) · [Patterns.dev](https://www.patterns.dev/vanilla/virtual-lists/) · [react-arborist](https://github.com/jameskerr/react-arborist) · [LogRocket-arb](https://blog.logrocket.com/using-react-arborist-create-tree-components/) · [Viprasol](https://viprasol.com/blog/react-virtual-table/)

**Drag-drop:** [Pragmatic-repo](https://github.com/atlassian/pragmatic-drag-and-drop) · [Pragmatic-docs](https://atlassian.design/components/pragmatic-drag-and-drop) · [core-pkg](https://atlassian.design/components/pragmatic-drag-and-drop/core-package/) · [npm](https://www.npmjs.com/package/@atlaskit/pragmatic-drag-and-drop) · [files-discussion](https://github.com/atlassian/pragmatic-drag-and-drop/discussions/139) · [PkgPulse](https://www.pkgpulse.com/guides/dnd-kit-vs-react-beautiful-dnd-vs-pragmatic-drag-drop-2026) · [RAC-DnD](https://react-spectrum.adobe.com/react-aria/dnd.html) · [useDrop](https://react-spectrum.adobe.com/react-aria/useDrop.html) · [useDrag](https://react-spectrum.adobe.com/react-aria/useDrag.html) · [collection](https://react-spectrum.adobe.com/react-aria/useDraggableCollection.html) · [Zoer](https://zoer.ai/posts/zoer/best-react-drag-drop-libraries-comparison) · [dnd-kit-virt](https://github.com/clauderic/dnd-kit/discussions/1372) · [SIDP](https://smart-interface-design-patterns.com/articles/drag-and-drop-ux/) · [LogRocket](https://blog.logrocket.com/ux-design/drag-and-drop-ui-examples/) · [SubUX](https://subux.pro/guides/article/accessible-drag-and-drop) · [Eleken](https://www.eleken.co/blog-posts/drag-and-drop-ui) · [MDN](https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API/File_drag_and_drop)

**Motion:** [layout](https://motion.dev/docs/react-layout-animations) · [docs](https://motion.dev/docs/react) · [AutoAnimate](https://awesome-react.dev/library/auto-animate) · [BuildUI](https://buildui.com/recipes/animated-list) · [theodorusclarence](https://theodorusclarence.com/blog/list-animation) · [PkgPulse](https://www.pkgpulse.com/guides/best-react-animation-libraries-2026) · [Inhaq](https://inhaq.com/blog/framer-motion-complete-guide-react-nextjs-developers) · [Nan.fyi](https://www.nan.fyi/magic-motion) · [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion)

**Skeleton:** [MatSimon](https://www.matsimon.dev/blog/simple-skeleton-loaders) · [jQueryScript](https://www.jqueryscript.net/blog/best-skeleton-loader.html) · [FreeFrontend](https://freefrontend.com/css-skeleton-loadings/) · [CodePen](https://codepen.io/maoberlehner/pen/bQGZYB) · [Syncfusion](https://ej2.syncfusion.com/react/documentation/skeleton/shimmer-effect)

**Bulk actions:** [Eleken](https://www.eleken.co/blog-posts/bulk-actions-ux) · [UXDWorld](https://uxdworld.com/best-practices-for-providing-actions-in-data-tables/) · [UXMovement](https://uxmovement.substack.com/p/a-better-ux-approach-to-bulk-actions) · [PatternFly](https://www.patternfly.org/patterns/bulk-selection/) · [DEV](https://dev.to/talissoncosta/bulkactionbar-part-1-the-ux-micro-interactions-that-make-bulk-actions-feel-intuitive-3eb4) · [Soul](https://soul.emplifi.io/latest/components/components/bulk-action-bar/usage-UJL5kHLb) · [SaaSInterface](https://saasinterface.com/components/bulk-actions/) · [Spectrum](https://spectrum.adobe.com/page/action-bar/)

**Empty states:** [Eleken](https://www.eleken.co/blog-posts/empty-state-ux) · [Carbon](https://carbondesignsystem.com/patterns/empty-states-pattern/) · [NN/g](https://www.nngroup.com/articles/empty-state-interface-design/) · [PatternFly](https://www.patternfly.org/components/empty-state/design-guidelines/) · [Setproduct](https://www.setproduct.com/blog/empty-state-ui-design) · [Toptal](https://www.toptal.com/designers/ux/empty-state-ux-design) · [SAP](https://www.sap.com/design-system/fiori-design-web/v1-96/foundations/best-practices/global-patterns/designing-for-empty-states) · [Mobbin](https://mobbin.com/glossary/empty-state)

**Keyboard / a11y:** [ARIA-Grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) · [Tableau](https://help.tableau.com/current/pro/desktop/en-us/access_keyboard_navigation.htm) · [DataTables](https://datatables.net/extensions/buttons/examples/initialisation/keys.html) · [Excel](https://support.microsoft.com/en-us/office/keyboard-shortcuts-in-excel-1798d9d5-842a-42b8-9c99-9b7213f0040f) · [Workato](https://docs.workato.com/data-tables/user-interface/keyboard-shortcuts.html) · [Wikipedia](https://en.wikipedia.org/wiki/Table_of_keyboard_shortcuts)

**Typography / Inter:** [Inter](https://madegooddesigns.com/inter-font/) · [pairings](https://madegooddesigns.com/inter-font-pairing/) · [DSguide](https://www.designsystems.com/typography-guides/) · [FZT](https://www.fourzerothree.in/p/typography-system) · [LogRocket-Linear](https://blog.logrocket.com/ux-design/linear-design-ui-libraries-design-kits-layout-grid/) · [Nadcab](https://www.nadcab.com/blog/apple-human-interface-guidelines-explained)

**React upload:** [Medium](https://medium.com/@dlrnjstjs/the-complete-react-file-upload-guide-from-drag-drop-to-progress-tracking-b2edb40016c2) · [Transloadit](https://transloadit.com/devtips/implementing-drag-and-drop-file-upload-in-react/) · [react.wiki](https://react.wiki/hooks/file-upload-hook/) · [BezKoder](https://www.bezkoder.com/react-drag-drop-file-upload/) · [Codewolfy](https://codewolfy.com/building-a-modern-file-upload-dropzone-in-react-with-dragdrop/) · [PrimeReact](https://primereact.org/fileupload/)
