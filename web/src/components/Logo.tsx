/**
 * Doc-Hub mark — an ink rounded square holding a stack of versioned
 * documents (the registry motif). Source of truth: `logo.svg` (repo root)
 * + web/public/favicon.svg. `currentColor` paints the square so callers
 * flip ink/amber via `color`; the document stack paints in `--mark-fg`
 * (the brand paper cream) so it reads opposite the square in any theme.
 * The current (front) sheet carries ink text lines painted in
 * `currentColor` so they track the square.
 *
 * Built from primitives (three offset sheets + text bars) rather than a
 * single path so the geometry stays editable and renders cleanly at
 * favicon size.
 */
export function Logo({ size = 38, className }: { size?: number; className?: string }) {
  return (
    <svg
      viewBox="0 0 38 38"
      width={size}
      height={size}
      role="img"
      aria-label="Doc-Hub"
      className={className}
      style={{ display: "block" }}
    >
      <defs>
        <clipPath id="dh-mark-clip">
          <rect width="38" height="38" rx="10" />
        </clipPath>
      </defs>
      <g clipPath="url(#dh-mark-clip)">
        <rect width="38" height="38" fill="currentColor" />
        {/* Stack of versioned documents — the registry. The paper fill
            stays opposite the square's currentColor whichever theme is
            active; older versions peek behind at reduced opacity. */}
        <g fill="var(--mark-fg, var(--paper, #F5F3EE))">
          <rect x="15" y="6" width="15" height="18" rx="2.5" fillOpacity="0.4" />
          <rect x="12" y="8.5" width="15" height="18" rx="2.5" fillOpacity="0.68" />
          <rect x="9" y="11" width="15" height="18" rx="2.5" />
        </g>
        {/* Text lines on the current version — paint in the square colour
            (ink) so they read as body text on the paper sheet. */}
        <g fill="currentColor">
          <rect x="11.5" y="16" width="10" height="1.6" rx="0.8" />
          <rect x="11.5" y="19.5" width="10" height="1.6" rx="0.8" />
          <rect x="11.5" y="23" width="6.5" height="1.6" rx="0.8" />
        </g>
      </g>
    </svg>
  );
}

/** The wordmark — "Doc-Hub", matching the logo's Inter wordmark.
 * `tone="rail"` inherits `currentColor` so the wordmark follows the
 * active-text colour of the dark sidebar; default is ink-on-paper. */
export function Wordmark({ tone = "default" }: { tone?: "default" | "rail" }) {
  const isRail = tone === "rail";
  return (
    <span style={{ display: "inline-block", lineHeight: 1 }}>
      <span
        style={{
          fontFamily: "var(--font-display)",
          fontWeight: 600,
          fontSize: 18,
          letterSpacing: "-0.015em",
          display: "block",
          color: isRail ? "inherit" : "var(--ink)",
        }}
      >
        Doc-Hub
      </span>
    </span>
  );
}
