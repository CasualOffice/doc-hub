# 01 — Reference SPAs: What Makes Premium Dashboards Feel Premium in 2026

**Audience:** the engineer redesigning Casual Drive's SPA (React 19 + Vite 7 + Tailwind v4 + TS).
**Purpose:** harvest concrete, copy-able patterns from nine best-in-class dashboards / list apps. Builds on `docs/research/04-polish-principles.md` — this brief is the *reference set* those rules pointed at.
**Scope:** visible-craft and interaction-craft. Performance architecture noted only where it shapes the UI.

---

## TL;DR

- The new monoculture: **near-monochrome canvas, single accent, hairline borders instead of shadows, ~32 px rows, 13 px text.** Linear, Vercel, Stripe, Notion, Raycast converge on the same grammar.
- **13 px is the new 14 px** for dense rows. 14–15 px is body. Display-cut faces (Inter Display, Geist, Söhne, SF Pro Display) carry headings with negative tracking.
- **Sidebar dimmer than canvas, content brighter than sidebar.** Linear's 2024 refresh — the single most reproducible cross-app pattern.
- **One accent does all the work.** Linear purple, Vercel near-black, Stripe `#533afd`, Raycast red. Appears in `selected` / `primary CTA` / `focus ring` and nowhere else.
- **Hairlines (#ebebeb) replace shadows for in-plane separation.** Shadows reserved for elevation only.
- **Optimistic UI is non-negotiable.** Linear writes locally → re-renders synchronously → queues to server. No spinners in the row.
- **`cmdk` (Paco Coursey) is the de facto Cmd-K component** — Linear, Vercel, Raycast, Sourcegraph all run on it.
- **Drive should ship:** Linear-density rows (32 px / 13 px), Geist tokens (`#fafafa` / `#171717` / `#ebebeb`), Stripe-style tables (no zebra, hover-only), Notion-warm sidebar greys, Raycast search-as-you-type.

---

## 1. Linear — Polish Benchmark

**Surfaces:** issues list, sign-in, command palette, settings, Inbox / Triage.

**Polish hooks:**
- **Sidebar dimmer than canvas.** The 2024 refresh "made the navigation sidebar dimmer in the updated interface, allowing the main content area to take precedence" ([part II](https://linear.app/now/how-we-redesigned-the-linear-ui)). The 2026 refresh kept pushing — "less visual noise, clearer structure, calmer UI" ([UI refresh changelog](https://linear.app/changelog/2026-03-12-ui-refresh)).
- **~32 px rows / 13 px text / 24 px line-box.** ([linear.app tokens, FontOfWeb](https://fontofweb.com/tokens/linear.app))
- **Compact tabs** with rounded corners and smaller icons; **reduced icon usage** with colored team-icon backgrounds removed ([part II](https://linear.app/now/how-we-redesigned-the-linear-ui)).
- **Warm grey shift.** "Old palette was cool, blue-ish; aim was to inch toward a warmer gray that still feels crisp" ([calmer interface](https://linear.app/now/behind-the-latest-design-refresh)).
- **Optimistic UI by architecture.** IndexedDB → MobX in-memory pool → synchronous re-render → background WebSocket sync ([performance.dev](https://performance.dev/how-is-linear-so-fast-a-technical-breakdown), [Vinta](https://www.vintasoftware.com/lessons-learned/hows-linear-so-fast-a-technical-breakdown)). Speed is a design decision.
- **Keyboard discoverability.** Every hover-able action shows its shortcut in a tooltip ([925studios breakdown](https://www.925studios.co/blog/linear-design-breakdown-saas-ui-2026)).

**Steal:** sidebar one notch dimmer than canvas; 32 px row × 13 px text × tabular nums; tooltip + mono shortcut chip on every action; optimistic write path for rename/move/star/delete from v1; warm zinc not blue slate.

**Anti-pattern:** density without consistent row rhythm reads cluttered — Linear's pre-2024 lesson.

---

## 2. Vercel Dashboard — Tailwind-Flavoured Dense

**Surfaces:** projects list, deployment detail, team switcher, sign-in. New dashboard default since early 2026 ([rollout](https://vercel.com/changelog/dashboard-navigation-redesign-rollout)).

**Polish hooks:**
- **Near-monochrome.** Off-white canvas `#fafafa`, near-black text/fill `#171717` (never pure `#000`), hairline border `#ebebeb` ([SeedFlip Geist breakdown](https://seedflip.co/blog/vercel-design-system), [Geist colors](https://vercel.com/geist/colors)).
- **Hairlines, not shadows.** `#ebebeb` is the default border for cards, nav, inputs, dividers — "carrying structural separation in place of shadows" ([SeedFlip](https://seedflip.co/blog/vercel-design-system)).
- **Geist Sans + Geist Mono.** Type scale `12 / 14 / 16 / 18 / 24 / 32 / 48 / 64`. Letter-spacing `-0.02em` at body sizes scaling to `-0.06em` at display ([Geist typography](https://vercel.com/geist/typography)).
- **Resizable/collapsible sidebar.** Redesign cut First Meaningful Paint by 1.2 s ([dashboard redesign](https://vercel.com/blog/dashboard-redesign)).
- **Mono everywhere it's data** — IDs, hashes, env vars. The typeface signals "this is data".
- **Rauno-level micro-craft.** Staff Design Engineer Rauno Freiberg documents the philosophy on [rauno.me/craft](https://rauno.me/craft) and [interfaces.rauno.me](https://interfaces.rauno.me/): "if a UI only works 80% of the time, the perception of quality breaks" ([Invisible Details](https://rauno.me/craft/interaction-design)).

**Steal:** Drive canvas `#fafafa`, text `#171717`, border `#ebebeb` — adopt verbatim. Mono for all numeric/ID columns. No row dividers between groups; 16 px gap instead. Adopt `interfaces.rauno.me` as PR-review criteria.

**Anti-pattern:** pure `#000` on pure `#fff` reads cheap. Never ship `text-black bg-white`.

---

## 3. Stripe Dashboard — Data-Table Done Right

**Surfaces:** payments table, customers list, sign-in, log detail drawer. Built on Stripe's internal **Sail** system ([Sail by Chase McCoy](https://portfolio.chsmc.org/sail), [Stripe UI components](https://docs.stripe.com/stripe-apps/components)).

**Polish hooks:**
- **Single typeface: Söhne.** Weight 300 for display headings, -0.03em tracking at 56 px ([Stripe Refero breakdown](https://styles.refero.design/style/48e5de76-05d5-4c4e-a269-c7c245b291ec)).
- **Deep Violet `#533afd` primary, Vibrant Orange `#ff6118` focus accent.** One CTA color, one focus color. Card surfaces with soft 6 px rounded corners.
- **No zebra; hover-only highlight.** Zebra is explicitly flagged as conflicting with hover + selection in modern tables ([NN/g data tables](https://www.pencilandpaper.io/articles/ux-pattern-analysis-enterprise-data-tables), [zebra vs hover](https://medium.com/@designbyfgs/do-zebra-striping-practices-in-table-ui-design-enhance-readability-or-create-visual-noise-5d98cc59f4fd)).
- **Row click → right drawer**, never full-page nav. Keeps table context anchored. `[unverified specifics]` from live dashboard.
- **8 px `elementGap`** for related controls (search + filter + segmented).
- **Sign-in: WebAuthn / passkeys first.** Magic-link verification in the background, no retype ([Eleken sign-up flows 2026](https://www.eleken.co/blog-posts/sign-up-flow)).

**Steal:** no zebra, hover-only highlight, no row dividers between groups, hairline below header. Row-click → right-drawer for file detail. Single accent. Sign-in: passkey-first, magic-link fallback, password tertiary.

**Anti-pattern:** Stripe's payments table shows 10+ columns by default → horizontal scroll. Drive: 4–6 columns max, rest in detail drawer.

---

## 4. Notion (native macOS + web) — Block UI, Chrome Lessons

**Surfaces:** sidebar, page hierarchy, file uploads, share modal.

**Polish hooks:**
- **NotionInter everywhere.** 16 px body @ 400; 20 px+ headings @ 500/600; 14 px captions ([DesignMD Notion benchmark](https://designmd.cc/benchmarks/notion)).
- **Strict 4 px scale: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64.** No magic numbers ([DesignMD](https://designmd.cc/benchmarks/notion)).
- **Warm greys.** System fonts via macOS, "warm grays replace harsh blacks, keeping the reading experience soft" ([sidebar breakdown](https://medium.com/@quickmasum/ui-breakdown-of-notions-sidebar-2121364ec78d)).
- **Sidebar icons in fixed 22 px columns.** Hierarchy carried by indentation alone — not by icon size or weight.
- **Drag handles appear on hover only**, never persistent. Removes 90% of chrome at rest.
- **One primary button: `#097fe8`, 4 px radius, white text.** Everywhere.
- **Share modal:** single column, permission row per recipient (dropdown), copy-link footer. No tabs.

**Steal:** enforce the 4 px spacing scale in lint. Drag handles on row-hover only. Share modal layout copy-pasted: single column, permission rows, copy-link footer. Sidebar icons in a fixed 22 px column.

**Anti-pattern:** Notion's web app historically had slow page transitions. Drive: every nav < 100 ms (optimistic).

---

## 5. Figma — The File Browser to Beat

**Surfaces:** Teams → Projects → Files, file grid, recents, sign-in.

**Polish hooks:**
- **Three-level nav: teams (sidebar) → projects (sidebar children) → files (grid)** ([file browser guide](https://help.figma.com/hc/en-us/articles/14381406380183-Guide-to-the-file-browser)). Breadcrumb mirrors sidebar position exactly.
- **Grid default, list toggle.** Thumbnails dominate; metadata small below. (Drive should invert: list default, grid toggle — Drive is mixed-content, not creative.)
- **Drag between projects** across the full sidebar tree without modal ([file org](https://bootcamp.uxdesign.cc/figma-files-organization-for-product-design-teams-3cfc13296be9)).
- **Avatar stacks on file rows** with active collaborators. Doubles as the "shared" indicator.
- **Empty project state:** muted line illustration + one sentence + one button. Illustration *never* colorful.

**Steal:** list-default with grid-toggle (persist per-folder). Drag from row to sidebar folder; drop target shows 2 px accent border on hover. Avatar stack in row when shared. Breadcrumb mirrors sidebar.

**Anti-pattern:** Figma's grid tiles (~200 px) make sparse folders look broken. Drive's grid tiles: ~160 px square.

---

## 6. Dropbox Web — Big-Table File Manager

**Surfaces:** file browser, action bar, share modal, file preview.

**Polish hooks:**
- **Expandable folder tree in left nav.** 2023–24 redesign added "an expandable folder tree for quicker content access" ([TechSpot on Dropbox redesign](https://www.techspot.com/news/100467-dropbox-rolls-out-redesigned-web-interface-releases-new.html)).
- **Selection-aware action bar.** Empty selection → "Upload / New folder / Record"; selected file → "Share / Move / Rename / Delete / Get link" ([GoodUX redesign intro](https://goodux.appcues.com/blog/dropbox-redesign)).
- **Inline file previews** with edit-in-place for PDFs and images.
- **Share modal:** per-recipient permission (Editor / Viewer dropdown) + copy-link row + Settings cog for link permissions.

**Steal:** action bar morphs on selection. Expandable folder tree as the sidebar spine; preserve disclosure state across nav. Inline preview panel; generic "preview unavailable" + metadata for unknown types.

**Anti-pattern:** Dropbox crams promotional banners ("Try Dash!", "Upgrade") into the browser chrome. Drive: zero in-product upsells. Cheap.

---

## 7. Arc Browser — Polish by Restraint

**Surfaces:** sidebar, Command Bar (`Cmd-T`), Little Arc, Spaces.

**Polish hooks:**
- **Vertical sidebar replacing horizontal tabs.** "Horizontal space is premium; vertical space is abundant" — long tab titles become readable ([Blake Crosley](https://blakecrosley.com/guides/design/arc), [Refine on Arc](https://refine.dev/blog/arc-browser/)).
- **Command Bar as the only address-bar.** `Cmd-T` opens universal search across tabs/history/bookmarks/actions ([Wikipedia](https://en.wikipedia.org/wiki/Arc_(web_browser))).
- **Little Arc:** stripped, chromeless quick-lookup window. Same product, two intensities.
- **Animation discipline.** Most chrome transitions 200 ms; sidebar fade sub-100 ms; hover/press instant. Polish by *not* animating most things.

**Steal:** `Cmd-K` as universal entry (use [cmdk by Paco Coursey](https://cmdk.paco.me/) — Linear / Vercel / Raycast / Sourcegraph all run on it). Optional "compact mode" — hide sidebar + details, full-screen list. Sidebar ~240 px expanded / 52 px collapsed, persistent per-user.

**Anti-pattern:** Josh Miller admitted Arc was "too different, with too many new things to learn, for too little reward" ([TechCrunch on Dia](https://techcrunch.com/2025/11/03/dias-ai-browser-starts-adding-arcs-greatest-hits-to-its-feature-set/)). Polish must not require re-learning canonical patterns.

---

## 8. Raycast — Cmd-K as the Whole App

**Surfaces:** root list, extension store, search bar as primary surface.

**Polish hooks:**
- **Search-as-you-type is primary.** Search bar focused on open; fuzzy filter on title + keywords client-side ([Raycast List docs](https://developers.raycast.com/api-reference/user-interface/list)).
- **Single-line rows.** Icon (16/20 px) + title + dimmed right-aligned subtitle + accessory. ~32 px row matches Linear.
- **Action panel (`Cmd-K`) per item.** `Enter` for primary; `Cmd-K` shows *all* actions for the highlighted item, each shortcut-labelled. The keyboard pattern Drive should adopt for file rows.
- **No empty state on root** — show recents instead.

**Steal:** file rows behave like Raycast list items — `Enter` opens, `Cmd-K` reveals all actions. Pre-focus global search on dashboard load. Title left, metadata mono right. Recents-as-first-screen — never show "0 files" pane.

**Anti-pattern:** Raycast is keyboard-only on native macOS. On web, the keyboard-only assumption fails for new users. Drive: keep hover states, right-click menus, kebab on each row. Keyboard is *parallel*, never *required*.

---

## 9. 1Password 8 — List Density Done Reluctantly

**Surfaces:** sign-in, vault list, item detail.

**Polish hooks:**
- **Knox design language** across web / mobile / desktop ([Knox case study by Alice Liao](https://aliceliao.com/work/knox), [TechTimes on Knox](https://www.techtimes.com/articles/268112/20211117/1password-introduces-enhanced-security-privacy-features-microsoft-windows-password-manager.htm)).
- **Semantic color tokens.** "A semantic token carries a role rather than a value — `color-surface-base`, `color-text-primary`" ([Muz.li on dark-mode systems](https://muz.li/blog/dark-mode-design-systems-a-complete-guide-to-patterns-tokens-and-hierarchy/)).
- **Sign-in chunked:** Secret Key + Master Password + biometrics across two screens, one decision per screen.
- **Three-pane layout:** sidebar (vaults / categories) → list (items in scope) → detail (selected).

**Steal:** three-pane shell as Drive default — sidebar / list / right detail drawer (closed by default, opens on row-click or `Cmd-I`). Semantic CSS variables; never raw hexes in components. Sign-in chunked across screens, WebAuthn first.

**Anti-pattern:** 1P8 cut list density to ~75% of 1P7 and got publicly punished — "users felt it didn't significantly improve clarity while reducing the amount of information visible without scrolling" ([1P community](https://1password.community/discussion/122677/item-list-information-density-in-1pw8)). Drive: land on Linear's 32 px density, not 1P8's 40+.

---

## Synthesis — The Converged Grammar

Patterns appearing in 6+ of 9 references — safe defaults for premium 2026 SPA design.

### Type rhythm

| Role | Size | Weight | Tracking | Used in |
|---|---|---|---|---|
| Page title | 20–24 px | 600 | -0.02 em | Linear, Vercel, Notion |
| Section title | 15–17 px | 600 | -0.01 em | All |
| Body | 14–15 px | 400 | 0 | Vercel, Stripe, Notion |
| **Dense list row** | **13 px** | **400** | **0** | **Linear, Raycast, Vercel** |
| Metadata / caption | 11–12 px | 400–500 | +0.005 em on UPPERCASE | All |
| Mono numeric | 12–13 px | 400 | 0 | Vercel, Stripe, Linear |

**Weight contrast:** 400 body / 500 emphasis / 600 headings. Bold (700) reserved for marketing. Max three weights per screen.

### Spacing rhythm

Notion published the canonical scale: **4, 8, 12, 16, 20, 24, 32, 40, 48, 64** ([DesignMD](https://designmd.cc/benchmarks/notion)). Linear / Vercel / Stripe ship from the same set. Drive's tokens already match — enforce in lint.

Inside a row: 8 px icon→label, 12 px label→metadata. Inside a card: 16 / 24 px padding. Between sections: 24 / 32 / 48 px.

### Colour discipline

- **Canvas off-white, not pure white.** Vercel `#fafafa`, Notion `#f6f5f4`.
- **Text near-black, never pure black.** `#171717` calibrated; `#000` exposes anti-aliasing.
- **Hairlines `#ebebeb` (light) / `rgba(255,255,255,0.08)` (dark)** replace shadows for in-plane separation.
- **One accent does all the work** — `selected`, `primary CTA`, `focus ring`. Linear purple, Vercel near-black, Stripe `#533afd`, Raycast red.
- **Semantic tokens, never raw hexes** ([Muz.li dark mode](https://muz.li/blog/dark-mode-design-systems-a-complete-guide-to-patterns-tokens-and-hierarchy/)).
- **Dark mode paired, not inverted.** "Build paired color scales from the ground up… extending them using LCH for perceptual uniformity" ([LogRocket on linear dark mode](https://blog.logrocket.com/how-do-you-implement-accessible-linear-design-across-light-and-dark-modes/), [Chyshkala](https://chyshkala.com/blog/why-linear-design-systems-break-in-dark-mode-and-how-to-fix-them)).

### Motion budget

- **Hover / press / focus: 80–120 ms** (sub-100 reads instant).
- **UI transitions: 150–250 ms.**
- **Full-screen: 400 ms max.**
- **Default curve `cubic-bezier(0.32, 0.72, 0, 1)`**; spring for direct manipulation.
- **Don't animate everything.** Rauno: "only animate when it clarifies cause & effect or when it adds deliberate delight" ([interfaces.rauno.me](https://interfaces.rauno.me/)). The macOS right-click menu only animates *out*, not in — frequent users feel the in-animation as latency. Drive should treat right-click, kebab, and tooltips the same way.

### Surface treatment

- **Hairlines do the work shadows used to.** Shadows reserved for elevation (popover / modal / drawer).
- **Shadows when used: soft, low-alpha, large blur.** `0 8px 24px /0.08` popover; `0 24px 60px /0.16` modal. Never `0 2px 4px /0.5`.
- **Vibrancy (`backdrop-blur`) on ≤1 surface per screen** ([NN/g glassmorphism](https://www.nngroup.com/articles/glassmorphism/)).

### Density

| App | Row height | Body | Notes |
|---|---|---|---|
| Linear | ~32 px | 13 px | The benchmark |
| Raycast | ~32 px | 13 px | Same, on native |
| Vercel | ~36 px | 14 px | Slightly looser |
| Stripe | ~40 px (tables) | 14 px | Tables run taller for click affordance |
| Notion | ~28 px sidebar / 36 px page rows | 14 / 16 px | Sidebar denser than content |
| Dropbox | ~44 px | 14 px | Looser; mouse-first |
| 1P8 | ~40 px | 14 px | Cautionary tale — too loose |

**Drive target: 32 px file rows / 13 px text, with per-user "comfortable" toggle to 40 / 14.** Ship one density at v1 (Sonoma System Settings warning in `04-polish-principles.md` §12).

### Empty states

- **One muted line illustration + one sentence + one button** ([SaaSUI Linear empty state](https://www.saasui.design/pattern/empty-state/linear)).
- **Empty search ≠ empty folder ≠ first-launch.** Each has own copy/CTA.
- **Raycast inversion:** no empty state on root — show recents.

### Focus rings

WCAG 2.4.13 requires ≥2 CSS px and ≥3:1 contrast against both element and background ([AllAccessible WCAG 2.4.13](https://www.allaccessible.org/blog/wcag-2413-focus-appearance-guide), [Sara Soueidan](https://www.sarasoueidan.com/blog/focus-indicators/)).

- **`box-shadow`, not `outline`** — outline ignores `border-radius` ([interfaces.rauno.me](https://interfaces.rauno.me/)).
- **`:focus-visible` only** so mouse users don't see it.
- **Double-ring for contrast:** `0 0 0 2px var(--bg-canvas), 0 0 0 4px var(--accent)` — inner matches background, outer is accent. [UK Parliament Design System](https://designsystem.parliament.uk/foundations/focus-state/) pattern, now standard.
- **Accent at ~60% opacity**, not full saturation.

---

## Patterns to Adopt Verbatim for Drive

PR-review checklist. Each item is a concrete spec.

### Layout
- [ ] **Three-pane shell:** sidebar (240 / 52 px collapsed) — file list — right detail drawer (closed by default; opens on row-click or `Cmd-I`).
- [ ] **Sidebar background one notch dimmer than canvas.**
- [ ] **Expandable folder tree** in sidebar; disclosure state persisted per-user.
- [ ] **Breadcrumb in file-list header mirrors sidebar position exactly.**

### Type
- [ ] Inter (web) / system-ui (Apple); Inter Display for headings ≥17 px.
- [ ] Scale `11 / 12 / 13 / 14 / 15 / 17 / 20 / 24 / 30`.
- [ ] **Tabular numerals** on every numeric column.
- [ ] Mono (JetBrains Mono or Geist Mono) for file size, hash, ID.
- [ ] Weights 400 / 500 / 600 only.

### Density
- [ ] **File row: 32 px high, 13 px text, 8 px icon→label, 12 px label→metadata.**
- [ ] Sidebar item: 28 px high, 13 px text.
- [ ] Comfortable toggle: 40 + 14. Ship one density at v1.

### Colour
- [ ] Canvas `#fafafa` light / `#0a0a0b` dark. Never pure `#fff` or `#000`.
- [ ] Text `#171717` light / `#f4f4f5` dark.
- [ ] Hairline `#ebebeb` light / `rgba(255,255,255,0.08)` dark.
- [ ] Single accent at `selected` / `primary CTA` / `focus ring` only.
- [ ] Paired dark scale, not invert.

### Borders & shadows
- [ ] Hairlines for in-plane separation; no shadows on resting cards.
- [ ] Shadow scale: `0 4px 12px /0.06` popover, `0 8px 24px /0.08` modal, `0 24px 60px /0.16` drawer.
- [ ] Vibrancy on ≤1 surface per screen.

### Motion
- [ ] Hover / press / focus 100 ms `cubic-bezier(0.32, 0.72, 0, 1)`.
- [ ] Popover / drawer 200 ms same curve.
- [ ] Page-level 300 ms max.
- [ ] Springs for direct manipulation: `{stiffness: 400, damping: 30}` snappy.
- [ ] `prefers-reduced-motion` honoured everywhere.
- [ ] Right-click menus, kebab menus, tooltips open instantly (no enter animation); close gently.

### Focus ring
- [ ] Double box-shadow: `0 0 0 2px var(--bg-canvas), 0 0 0 4px color-mix(in srgb, var(--accent) 60%, transparent)`.
- [ ] `:focus-visible` only.
- [ ] Never `outline: none` without a replacement.

### Hover treatment
- [ ] Row hover: 4–6% accent overlay (light) / 6–8% white overlay (dark). No border change.
- [ ] Drag-handle on hover only, never persistent.
- [ ] Cursor: `pointer` for nav, `default` for buttons, `text` for editable, `grab`/`grabbing` for handles.

### Tables / lists
- [ ] **No zebra.** Hover-only highlight.
- [ ] No row dividers inside a group; hairline below header; 16 px gap between groups.
- [ ] Selection: 10–18% accent overlay + 1 px left-edge accent rule.
- [ ] Avatar stack on shared rows.
- [ ] **Row click → right drawer.** Never full-page nav.

### Keyboard
- [ ] **`cmdk`-powered Cmd-K palette** as universal action surface.
- [ ] Every important action has a shortcut, advertised in tooltip with mono chip.
- [ ] Per-row `Cmd-K` action menu (Raycast pattern): `Enter` primary, `Cmd-K` all.
- [ ] Global search pre-focused on root.
- [ ] `Esc` always closes nearest dismissible surface.
- [ ] Tab order mirrors visual reading order.

### Optimistic UI
- [ ] Rename, move, star, delete, create-folder all optimistic.
- [ ] Toast on optimistic success with undo (sonner): "Moved to Archive — Undo".
- [ ] No spinners on optimistic actions. Spinners only for ≥1 s finite tasks (uploads, exports).

### Empty states
- [ ] First-launch: muted line illustration + one sentence + one button.
- [ ] Empty folder: small icon + "This folder is empty." + no button.
- [ ] Empty search: "No files matched <query>." + Clear link.
- [ ] Root with recents: show last 10 used, never empty pane.

### Sign-in
- [ ] **Passkey-first;** magic-link fallback; password tertiary behind disclosure.
- [ ] Multi-field auth chunked across screens (1P8 pattern): one decision per screen.
- [ ] Sign-in surface = same canvas + hairlines + accent as app. Continuity.

### Anti-clichés
- [ ] No gradient primary buttons.
- [ ] No pure-black-on-pure-white.
- [ ] No omnipresent glassmorphism (one blurred surface per screen max).
- [ ] No in-product upsells in browser chrome.
- [ ] No "Oops!" / "Whoops!"
- [ ] No 1P8-style over-spacing.

---

## Sources

**Reference apps:**
- Linear — [UI refresh 2026](https://linear.app/changelog/2026-03-12-ui-refresh) · [new Linear 2024](https://linear.app/changelog/2024-03-20-new-linear-ui) · [redesigned Linear UI part II](https://linear.app/now/how-we-redesigned-the-linear-ui) · [calmer interface](https://linear.app/now/behind-the-latest-design-refresh) · [linear.app tokens (FontOfWeb)](https://fontofweb.com/tokens/linear.app) · [925studios breakdown](https://www.925studios.co/blog/linear-design-breakdown-saas-ui-2026) · [performance.dev](https://performance.dev/how-is-linear-so-fast-a-technical-breakdown) · [Vinta](https://www.vintasoftware.com/lessons-learned/hows-linear-so-fast-a-technical-breakdown) · [SaaSUI empty state](https://www.saasui.design/pattern/empty-state/linear) · [Karri Saarinen on craft (Figma)](https://www.figma.com/blog/karri-saarinens-10-rules-for-crafting-products-that-stand-out/)
- Vercel — [Dashboard redesign](https://vercel.com/blog/dashboard-redesign) · [New side of Vercel](https://vercel.com/try/new-dashboard) · [Nav rollout 2026](https://vercel.com/changelog/dashboard-navigation-redesign-rollout) · [Geist intro](https://vercel.com/geist/introduction) · [Geist typography](https://vercel.com/geist/typography) · [Geist colors](https://vercel.com/geist/colors) · [SeedFlip Geist breakdown](https://seedflip.co/blog/vercel-design-system) · [Vercel design guidelines](https://vercel.com/design/guidelines)
- Stripe — [Stripe UI components](https://docs.stripe.com/stripe-apps/components) · [Stripe sign-in pattern](https://docs.stripe.com/stripe-apps/patterns/sign-in) · [Stripe Refero breakdown](https://styles.refero.design/style/48e5de76-05d5-4c4e-a269-c7c245b291ec) · [Sail by Chase McCoy](https://portfolio.chsmc.org/sail) · [Stripe accessible colors](https://stripe.com/blog/accessible-color-systems)
- Notion — [DesignMD Notion benchmark](https://designmd.cc/benchmarks/notion) · [UI breakdown of Notion's sidebar](https://medium.com/@quickmasum/ui-breakdown-of-notions-sidebar-2121364ec78d) · [Notion navigate sidebar](https://www.notion.com/help/navigate-with-the-sidebar) · [Notion sharing](https://www.notion.com/help/sharing-and-permissions)
- Figma — [File browser guide](https://help.figma.com/hc/en-us/articles/14381406380183-Guide-to-the-file-browser) · [Files and projects](https://help.figma.com/hc/en-us/articles/1500005554982-Guide-to-files-and-projects) · [Team / file org](https://www.figma.com/best-practices/team-file-organization/) · [Bootcamp file org](https://bootcamp.uxdesign.cc/figma-files-organization-for-product-design-teams-3cfc13296be9)
- Dropbox — [TechSpot 2024 redesign](https://www.techspot.com/news/100467-dropbox-rolls-out-redesigned-web-interface-releases-new.html) · [April 2024 updates](https://www.dropbox.com/product-updates/april-2024) · [GoodUX redesign intro](https://goodux.appcues.com/blog/dropbox-redesign) · [Dropbox link permissions](https://help.dropbox.com/share/set-link-permissions)
- Arc — [Wikipedia Arc](https://en.wikipedia.org/wiki/Arc_(web_browser)) · [Blake Crosley Arc](https://blakecrosley.com/guides/design/arc) · [Refine Arc](https://refine.dev/blog/arc-browser/) · [TechCrunch Dia](https://techcrunch.com/2025/11/03/dias-ai-browser-starts-adding-arcs-greatest-hits-to-its-feature-set/)
- Raycast — [Raycast List API](https://developers.raycast.com/api-reference/user-interface/list) · [Raycast UI API](https://developers.raycast.com/api-reference/user-interface) · [Raycast Store](https://www.raycast.com/store)
- 1Password 8 — [1P8 for Windows](https://1password.com/blog/1password-8-for-windows-is-here) · [1P8 list density (community)](https://1password.community/discussion/122677/item-list-information-density-in-1pw8) · [TechTimes on Knox](https://www.techtimes.com/articles/268112/20211117/1password-introduces-enhanced-security-privacy-features-microsoft-windows-password-manager.htm) · [Knox by Alice Liao](https://aliceliao.com/work/knox) · [Muz.li dark-mode systems](https://muz.li/blog/dark-mode-design-systems-a-complete-guide-to-patterns-tokens-and-hierarchy/)

**Craft / cross-cutting:**
- [rauno.me/craft](https://rauno.me/craft) · [Invisible Details of Interaction Design](https://rauno.me/craft/interaction-design) · [Web Interface Guidelines](https://interfaces.rauno.me/) · [Devouring Details](https://devouringdetails.com/) · [Mantlr: Stripe/Linear/Vercel premium UI](https://mantlr.com/blog/stripe-linear-vercel-premium-ui) · [Pixeldarts: four design principles](https://www.pixeldarts.com/en/post/four-design-principles-behind-stripe-linear-and-vercel) · [cmdk by Paco Coursey](https://cmdk.paco.me/) · [Build a Cmd-K palette](https://www.techinterview.org/post/3233475212/build-command-palette-cmd-k/)

**Focus & dark mode:**
- [Sara Soueidan on focus indicators](https://www.sarasoueidan.com/blog/focus-indicators/) · [AllAccessible WCAG 2.4.13](https://www.allaccessible.org/blog/wcag-2413-focus-appearance-guide) · [a11y-collective focus indicator](https://www.a11y-collective.com/blog/focus-indicator/) · [UK Parliament focus state](https://designsystem.parliament.uk/foundations/focus-state/) · [LogRocket linear in light/dark](https://blog.logrocket.com/how-do-you-implement-accessible-linear-design-across-light-and-dark-modes/) · [Chyshkala on linear dark mode](https://chyshkala.com/blog/why-linear-design-systems-break-in-dark-mode-and-how-to-fix-them)

**Tables / patterns:**
- [Pencil & Paper data tables](https://www.pencilandpaper.io/articles/ux-pattern-analysis-enterprise-data-tables) · [Refactoring UI: labels are a last resort](https://refactoringui.com/previews/labels-are-a-last-resort/) · [zebra striping analysis](https://medium.com/@designbyfgs/do-zebra-striping-practices-in-table-ui-design-enhance-readability-or-create-visual-noise-5d98cc59f4fd) · [NN/g empty states](https://www.nngroup.com/articles/empty-state-interface-design/) · [NN/g glassmorphism](https://www.nngroup.com/articles/glassmorphism/) · [Eleken sign-up flows 2026](https://www.eleken.co/blog-posts/sign-up-flow)
