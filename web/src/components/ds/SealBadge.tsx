/**
 * SealBadge — "the Seal" signature moment (ui-vision-2026 §6, moment #1).
 *
 * When a document's hash chain validates, a small glass seal badge plays a
 * one-shot amber specular SWEEP (a masked highlight crossing left→right) and
 * settles into a quiet mono caption — "Sealed · SHA-256 · verified". The one
 * place the delight budget is spent: ~600ms, the vision's overshoot easing
 * (`--ease-seal`), once, never decorative.
 *
 * The animation runs on MOUNT — callers remount with a changing `key` to
 * replay it on each successful verify. Under `prefers-reduced-motion: reduce`
 * the sweep + entrance are dropped and the badge renders static.
 *
 * `variant="static"` skips the sweep entirely for quiet, non-celebratory
 * reuse (e.g. a persisted "Sealed" marker) so the moment stays rare.
 */
import { ShieldCheck } from "lucide-react";

export function SealBadge({
  caption = "Sealed · SHA-256 · verified",
  variant = "seal",
  title,
}: {
  caption?: string;
  variant?: "seal" | "static";
  title?: string;
}) {
  return (
    <span
      className={variant === "seal" ? "cd-seal cd-seal--play" : "cd-seal"}
      title={title ?? caption}
      aria-label={title ?? caption}
    >
      <style>{`
        .cd-seal {
          position: relative;
          overflow: hidden;
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 3px 10px 3px 8px;
          border-radius: var(--radius-pill);
          background: var(--mat-thin);
          -webkit-backdrop-filter: blur(10px) saturate(var(--saturate));
          backdrop-filter: blur(10px) saturate(var(--saturate));
          border: var(--hairline-glass);
          box-shadow: var(--edge-hi), 0 0 0 1px var(--amber-glow-3);
          color: var(--status-verified-700);
          white-space: nowrap;
          line-height: 1;
        }
        @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
          .cd-seal { background: var(--glass-solid); }
        }
        @media (prefers-reduced-transparency: reduce) {
          .cd-seal {
            background: var(--glass-solid);
            -webkit-backdrop-filter: none;
            backdrop-filter: none;
          }
        }
        .cd-seal__icon { display: inline-flex; flex-shrink: 0; color: var(--accent); }
        .cd-seal__cap {
          font-family: var(--font-mono);
          font-size: var(--mono-xs);
          font-weight: var(--weight-medium);
          letter-spacing: 0;
        }
        /* The specular sweep — a masked amber highlight crossing once. */
        .cd-seal__sweep {
          position: absolute;
          inset: 0;
          border-radius: inherit;
          pointer-events: none;
          opacity: 0;
          background: linear-gradient(
            105deg,
            transparent 30%,
            var(--amber-glow-1) 46%,
            rgba(255, 255, 255, 0.55) 50%,
            var(--amber-glow-1) 54%,
            transparent 70%
          );
          transform: translateX(-130%);
          mix-blend-mode: screen;
        }
        /* Play = entrance overshoot + one-shot sweep (both fill so it settles). */
        .cd-seal--play {
          animation: cd-seal-in var(--dur-seal) var(--ease-seal) both;
        }
        .cd-seal--play .cd-seal__sweep {
          animation: cd-seal-sweep var(--dur-seal) var(--ease-seal) 100ms both;
        }
        @keyframes cd-seal-in {
          from { opacity: 0; transform: scale(0.92); }
          60%  { opacity: 1; }
          to   { opacity: 1; transform: scale(1); }
        }
        @keyframes cd-seal-sweep {
          0%   { opacity: 0; transform: translateX(-130%); }
          12%  { opacity: 1; }
          88%  { opacity: 1; }
          100% { opacity: 0; transform: translateX(130%); }
        }
        /* Reduced motion → static badge, no sweep, no entrance. */
        @media (prefers-reduced-motion: reduce) {
          .cd-seal--play { animation: none; }
          .cd-seal--play .cd-seal__sweep { animation: none; opacity: 0; }
        }
      `}</style>
      <span className="cd-seal__icon" aria-hidden>
        <ShieldCheck size={13} strokeWidth={1.5} />
      </span>
      <span className="cd-seal__cap">{caption}</span>
      {variant === "seal" && <span className="cd-seal__sweep" aria-hidden />}
    </span>
  );
}
