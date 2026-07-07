/**
 * SkeletonRow — a loading placeholder that mirrors the dense vault table's
 * column widths so the layout doesn't jump when rows land. Uses the
 * `.skeleton` shimmer utility (static under prefers-reduced-motion).
 */

/** Column template shared by the vault header, rows, and skeleton.
 * M6 relayout: 8 → 5 columns. Kind / Lock / Encryption dropped;
 * Version is conditional (empty placeholder cell when not
 * compliance-significant, so the grid stays aligned).
 * Tracks: select · name · version · updated · kebab. */
export const VAULT_GRID = "24px minmax(0,1fr) 56px 96px 32px";

/** Approx bar widths per column (name is widest). */
const BAR_WIDTHS = ["16px", "60%", "28px", "72px", "16px"];

export function SkeletonRow({ columns = BAR_WIDTHS.length }: { columns?: number }) {
  const widths = BAR_WIDTHS.slice(0, columns);
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: VAULT_GRID,
        alignItems: "center",
        height: 32,
        padding: "0 var(--space-3)",
        gap: "var(--space-3)",
        borderBottom: "1px solid var(--border-hair)",
      }}
    >
      {widths.map((w, i) => (
        <div
          key={i}
          className="skeleton"
          style={{ height: 10, width: w, borderRadius: "var(--radius-2xs)" }}
        />
      ))}
    </div>
  );
}
