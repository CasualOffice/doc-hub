/**
 * AmbientGround — UI M6 glass foundation (docs/design/ui-system-glass.md §3).
 *
 * A single fixed, GPU-cheap layer behind the whole app: the `--ground`
 * colour plus two large, slow-drifting radial blooms (`--ground-aurora-1/2`)
 * that the glass chrome blurs for real vibrancy. It sits at `z-index: -1`
 * behind every surface and never intercepts pointer events, so it is inert
 * for existing shell/vault/compliance surfaces stacked above it.
 *
 * All finish lives in the `.ambient-ground` class in `styles/tokens.css`:
 * the ground/aurora colours swap per theme (light "frosted paper" default,
 * dark hero) and the ~30s drift animation is disabled under
 * `prefers-reduced-motion: reduce`.
 */
export function AmbientGround() {
  return <div className="ambient-ground" aria-hidden="true" />;
}
