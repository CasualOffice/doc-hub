# 02 — Surface spec (v2)

Supersedes the relevant sections of [`02-surface.md`](./02-surface.md) for the Phase-1 SPA rebuild. Authoritative source for what each surface looks like.

**Inputs synthesized:**
- [`mockup-v1.html`](../ui-research/mockup-v1.html) + [`mockup-v2.html`](../ui-research/mockup-v2.html) — the user-supplied interactive references; visual + interaction truth
- [`ui-research/01-reference-spas.md`](../ui-research/01-reference-spas.md) — Linear/Vercel/Stripe/Notion/Figma/Dropbox/Arc/Raycast/1Password patterns
- [`ui-research/02-stack-pick.md`](../ui-research/02-stack-pick.md) — Radix + shadcn/ui + Motion + auto-animate + vaul + sonner + cmdk + react-hook-form + zod
- [`ui-research/03-sign-in-patterns.md`](../ui-research/03-sign-in-patterns.md) — sign-in field strategy + error UX + a11y
- [`ui-research/04-file-table.md`](../ui-research/04-file-table.md) — row heights / hover / focus / motion (still landing; cross-check on completion)
- [`research/04-polish-principles.md`](../research/04-polish-principles.md) — the 10 commandments stay in force

## 1 — Identity (the things that change from v1)

| Token | v1 (old) | v2 (new) |
|---|---|---|
| `--bg-canvas` | `#FAFAFA` cool grey | `#F2F0EA` warm paper |
| `--bg-default` (card) | `#FFFFFF` | `#FBFAF6` warm card |
| `--fg-default` (ink) | `#18181B` | `#1A1A1E` |
| `--fg-muted` | `#52525B` | `#3A3A42` (`ink-soft`) for medium; `#8A8A92` for muted; `#A6A6AD` for muted-2 |
| Accent | `#0A84FF` macOS blue | `#C8A45C` warm gold |
| Font (sans) | Inter only | **Hanken Grotesk** (body) |
| Font (display) | Inter Display | **Fraunces** (variable serif, headings + brand + numerals where typographic) |
| Motion ease | `cubic-bezier(0.32, 0.72, 0, 1)` | `cubic-bezier(.2, .8, .2, 1)` — snappier in/out |
| Focus ring | `0 0 0 4px rgba(10,132,255,.6)` | `outline: 2px solid var(--ink); outline-offset: 2px; border-radius: 6px` (global `:focus-visible`) |
| Card radius | `8px` (md) | `18px` (`--radius`) |
| Shadow | flat sm/md/lg/xl scale | `--shadow: 0 1px 2px /.04, 0 8px 30px /.05` + `--shadow-hover: 0 2px 4px /.06, 0 16px 44px /.10` (paired soft + atmosphere) |

**Why the palette change:** the warm paper + gold accent gives Drive editorial personality, separates it from "yet another macOS-blue SaaS dashboard", and aligns with the "Casual" brand. Reference brief #1 confirms Linear/Vercel/Stripe converge on neutral palettes; Drive's differentiator is the warm cream, not another dark-ink-with-blue-accent.

## 2 — Stack (locked from brief #2)

| Layer | Pick | Why |
|---|---|---|
| Component primitives | **Radix Primitives** (`radix-ui` 1.4.x umbrella) | Accessible, keyboard-first, every primitive we need; foundation of shadcn |
| Visual layer | **shadcn/ui** copy-paste components (CLI v4, Tailwind v4 + React 19 first-class) | Full polish headroom — code lives in our repo, edited to match the 10 commandments |
| Motion (general) | **Motion** (`motion` 12.x) with `LazyMotion(domAnimation)` + `m` components | ~4.6 kB initial; for layout/spring/gesture/exit |
| Motion (list reorder) | **`@formkit/auto-animate`** | Drop-in for the file-list reorder; cheaper than Motion's layout for this case |
| Toasts | **sonner** 2.x | Bottom-centered stacking, paired with our ink/gold style |
| Drawers (mobile sheet) | **vaul** 1.1.x | Drag-to-dismiss; mobile sidebar later |
| Command palette | **cmdk** 1.1.x | Phase-2 surface but install slot now |
| Forms | **react-hook-form** 7.x + **zod** 4.x | Sign-in + future settings |
| Icons | **lucide-react** | Inviolable rule — SVG only |
| Type | **`@fontsource-variable/fraunces`** + **`@fontsource-variable/hanken-grotesk`** | Self-host, no Google Fonts hop |

## 3 — Drive-tailoring (what mockup-v2 implies but Drive v0 strips)

The mockups are SaaS-flavored Google-Drive-shaped. Drive v0 is single-tenant + self-host. Tailor:

- **Drop "Upgrade plan" link** in the Storage card. Show actual disk/S3 quota from config (`DRIVE_STORAGE_QUOTA_GB`). When unset → "122 GB used" only, no percentage, no upsell.
- **Hide Shared + Starred nav items** in v0 (slots present, return empty / Phase-3 multi-user). Trash + Recent stay.
- **Drop "Owner" column in list view** for v0; restore when multi-user. List columns: Name, Modified, Size, Type.
- **Drop "Shared with" faces** from preview modal in v0; restore when multi-user.
- **Avatar** lives in the **sidebar bottom** (pinned, above Storage card) per surface §2 of the original spec — not in the top bar. The mockup's top-bar avatar is a defensible alternative, but the sidebar-bottom placement matches the polish principles and gives the top bar more breathing room. **Final call: sidebar bottom.**
- **Preview modal primary action is type-aware:**
  - `.xlsx` / `.ods` / `.csv` → "**Open in Casual Sheets**" (links out to `sheet.<host>` via WOPI handoff)
  - `.docx` → "**Open in Casual Editor**" (links out to `doc.<host>`)
  - Others (PDF/image/video/text) → "**Open preview**" inline (Phase 2 preview surface)
  - Opaque (everything else) → "**Download**"
  The "Download" button always exists as a secondary action.
- **All `Owner`-shaped sample data shows "You"** for v0 (no "Lena R." / "Sam K.").

## 4 — Surfaces

### Sign-in (replaces §13 of the original spec)

Reference: brief #3, mockup palette + typography (sign-in surface not in mockup; extrapolate).

- 360 px centred card, `--bg-default` background, `--shadow-md`, hairline border, `--radius-xl`.
- Logo (28 px Casual Drive mark in `--accent`) → Fraunces wordmark "Casual Drive" (semibold, `--text-xl`, `-0.01em` tracking) → Hanken muted subtitle "Sign in to continue."
- **Two stacked inputs**: Username (`autoFocus`, `autocomplete=username`), Password (`autocomplete=current-password`). Per brief #3, single-step preferred when only one account exists.
- Submit button full-width, ink-filled, paper text. Disabled until both fields non-empty.
- Error: inline below inputs in `--danger` text, 1 px border tint on the inputs, single horizontal shake on wrong credentials (8 px, 280 ms, `--ease`).
- Caps-lock helper text in `--fg-muted text-xs` when relevant.
- Microcopy: title "Casual Drive", subtitle "Sign in to continue.", error "Wrong username or password.", lockout "Too many attempts. Try again in 10 minutes."
- No "Forgot password" link in v0 (single-tenant). No "Sign up". No OAuth buttons.
- Background: solid `--bg-canvas` (paper). No marketing imagery in v0.
- `prefers-reduced-motion`: shake becomes a 50 ms opacity nudge.

### Sidebar (replaces §2)

| Region | Spec |
|---|---|
| **Brand row** | 48 px, Logo (38 px) + `<wm><span.c>Casual</span><span.d>Drive</span></wm>`. Fraunces "Casual" (500, 18 px) over uppercase letter-spaced "DRIVE" (500, 10 px, 4 px letter-spacing). |
| **"New" button** | Full-width ink-filled, `--radius` 14 px. Click → dropdown menu (Radix DropdownMenu, slide+fade 200 ms): **New folder** / **Upload files**. |
| **Nav (Library)** | Section label "LIBRARY" (10 px uppercase letter-spaced muted-2). Items: **My Drive** (item count badge), **Recent**, **Shared** (hidden v0), **Starred** (hidden v0). Active row = ink-filled, paper text, 2 px accent left-edge stripe. |
| **Nav (System)** | Section label "SYSTEM". Items: **Trash** (item count when non-empty), **Settings**. |
| **Avatar (pinned bottom)** | 40 px monogram circle, `--card` background, `--line` ring, opens Radix DropdownMenu (Account / Settings / Sign out with `⇧⌘Q` chord). |
| **Storage card (above avatar, also pinned bottom)** | `--card` background, `--radius` 18 px, `--line` border. Title "Storage" (13 px) + Fraunces percent (when quota set). Animated bar fills 0→% on mount (1.2 s `--ease`). Subtitle "122 GB of 200 GB used" (when quota set) or "122 GB used" (when not). No "Upgrade plan". |

Width 248 px. Right border `--line`. No vibrancy/backdrop-filter for v0 (defer to Phase-2 polish; cleaner without it).

### Top bar (replaces §3)

48 px tall. Layout: **search trigger** (left, faux-input that opens cmdk palette later — for now a real input that filters the current view), **view toggle** (Grid/List filled-on segmented), **(no avatar here)**.

### File-browser pane (replaces §4 + §5 + §7)

- **Head** (mockup-v2): optional **Back button** (34 px, shown when not at root), **breadcrumbs** (Fraunces 13 px muted, clickable, chevron 13 px separators), **Title row** (`<h1>` Fraunces 30 px semibold + count "12 items" in `--fg-muted text-sm`), **Sort dropdown** (right-aligned: Name / Last modified / Size; folders pinned first).
- **Quick Access section** (only shown at root, hidden when navigating into a folder or searching): "QUICK ACCESS" section-head + 4-column grid of qcards. Each qcard: 46 px thumbnail + name + meta.
- **Section-head pattern**: Fraunces 15 px regular muted "Quick access" / "All files" + flex-1 hairline rule on the right.
- **Stage**: animates on folder change (`@keyframes swap` opacity 0→1, translateY 8→0, 420 ms ease).
- **Grid view (default)**: `repeat(auto-fill, minmax(190px, 1fr))` gap 16 px. Each item:
  - Card: `--card` bg, `--radius`, `--shadow`, hover lifts to `--shadow-hover` + `translateY(-3px)` + border `--line-strong`.
  - 130 px thumbnail at top (procedural — paper for doc, sheet grid for spreadsheet, gradient for image, play+duration for video, folder glyph for folder, red bar header for PDF), with type-tinted background.
  - Bottom meta: 16 px Lucide type-icon + name (13.5 px medium) + sub `Type · Modified` (11.5 px muted).
  - Folder cards: no border between thumb and meta; hover reveals top-right chevron hint (open-into).
- **List view**: card with hairline-separated rows. Columns: Name (with 30 px lthumb + 13.5 px label) · Modified · Size · (Type implicit via icon). Hover: 3% ink overlay.
- **Empty state** (v2 mockup): centred column with 96 px illustration container (icon + plus, paper bg, muted-2 stroke), Fraunces 18 px title, Hanken 13.5 px subtitle.
  - Empty root (no files at all): "Your Drive is empty." / "Drop files here, or use the New button."
  - Empty folder: "This folder is empty." / "Drag files here or use the New button to add something."
  - Empty search: "No files match \"<q>\"." / "Try a different search."

### Preview modal (replaces §10.3)

- Radix Dialog. Backdrop: `rgba(26,26,30,0.42)` + `backdrop-filter: blur(6px)`.
- Modal: 1000 × 640 (clamped 90vh), `--card` bg, `--radius` 24 px, `--shadow-xl`, slide+scale entrance (translateY 14 → 0, scale .98 → 1, 320 ms `--ease`).
- Layout: two columns — **preview stage** (1fr, warm grey `#E7E4DC` bg, 46 px padding, file preview centered) + **detail sidebar** (320 px, `--card`, left border).
- Stage shows the file (or placeholder thumbnail for now), prev/next arrow buttons floating mid-edge.
- Detail sidebar:
  - Top-right Close (×) button
  - Title row: 22 px icon + Fraunces 19 px filename
  - Type meta (`Document · 182 KB` muted)
  - **Action row** (3 buttons): primary (type-aware, see Drive-tailoring above), Share (Phase 2), Star (icon-only). Buttons hover `translateY(-1px)` for primary; bg-change for secondary.
  - Details list: Type / Size / Modified / Location (Type/Modified/Size always; Owner hidden in v0).
- Keyboard: Esc close, ←/→ prev/next.

### Toasts (replaces §11)

- Sonner. Position: bottom-center stack. Bar: ink bg, paper text, gold check icon on success.
- Used for: upload completion, link copied, folder created, file moved/trashed, restore, sign-out-for-security.
- Lifetime: 4 s default, 8 s when there's an Undo action (deferred to Phase-2 trash flow).

## 5 — Motion budget (consolidated)

| Element | Property | Duration | Curve |
|---|---|---|---|
| Buttons / nav rows hover | bg | 180 ms | `--ease` |
| Buttons press (filled) | translateY(-1px) | 200 ms | `--ease` |
| Cards hover | translateY(-3px) + shadow + border | 300 ms | `--ease` |
| Stage swap (folder nav, search) | opacity + translateY(8→0) | 420 ms | `--ease` |
| Dropdown open | opacity + translateY(-6→0) | 200 ms | `--ease` |
| Modal open | opacity + (translateY(14→0), scale(.98→1)) | 280 ms / 320 ms | `--ease` |
| Toast in/out | opacity + translateY(12) | 300 ms | `--ease` |
| Storage bar fill | width 0→% | 1200 ms | `--ease` |
| Sign-in shake | translateX 0/-6/6/0 | 280 ms | `--ease` |
| Reveal sections on mount | opacity + translateY(10→0) | 600 ms | `--ease` |

`prefers-reduced-motion` → all of the above collapse to ≤50 ms opacity-only.

## 6 — What stays from the v1 spec

§2 keyboard model (arrows / Enter / Backspace / F2 / Cmd-A / Esc / letter-jump). §8 multi-select bar (Phase 2). §9 cmdk command palette (Phase 2). §12 drop zones + inline upload row. §14 recipient share page (Phase 2). §15 editor-handoff badge (replaced by type-aware primary action in preview modal).
