/**
 * Casual Drive mark — black rounded square with a negative-space crescent.
 * `currentColor` paints the square so callers can flip light/dark via `color`.
 * The crescent fill (`--mark-fg`) defaults to the brand paper cream.
 */
export function Logo({ size = 38, className }: { size?: number; className?: string }) {
  return (
    <svg
      viewBox="0 0 38 38"
      width={size}
      height={size}
      role="img"
      aria-label="Casual Drive"
      className={className}
      style={{ display: "block" }}
    >
      <defs>
        <clipPath id="cd-mark-clip">
          <rect width="38" height="38" rx="10" />
        </clipPath>
      </defs>
      <g clipPath="url(#cd-mark-clip)">
        <rect width="38" height="38" fill="currentColor" />
        <circle cx="16.5" cy="19.5" r="11.5" fill="var(--mark-fg, #F2F0EA)" />
        <circle cx="22.5" cy="16.5" r="11.5" fill="currentColor" />
        <circle cx="24.3" cy="14.6" r="1.5" fill="var(--mark-fg, #F2F0EA)" />
      </g>
    </svg>
  );
}

/** The wordmark — Fraunces "Casual" over uppercase letter-spaced "DRIVE". */
export function Wordmark() {
  return (
    <span style={{ display: "inline-block", lineHeight: 1 }}>
      <span
        style={{
          fontFamily: "var(--font-display)",
          fontWeight: 500,
          fontSize: 18,
          letterSpacing: "0.5px",
          display: "block",
          color: "var(--ink)",
        }}
      >
        Casual
      </span>
      <span
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 10,
          letterSpacing: "4px",
          textTransform: "uppercase",
          color: "var(--muted)",
          fontWeight: 500,
          marginTop: 3,
          display: "block",
        }}
      >
        Drive
      </span>
    </span>
  );
}
