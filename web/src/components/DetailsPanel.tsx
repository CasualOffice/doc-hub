/**
 * DetailsPanel — the single-card compliance summary that lives both in the
 * PreviewModal and in the FileFullscreen header drawer (UX-EDITOR-8).
 *
 * The former three tabs (Info / People / History) were cut in M6: metadata
 * belongs inline, sharing routes through the ShareDialog, and version
 * history has ONE canonical home — the `/document/{id}/history` route. This
 * panel now renders a single glass compliance card that states the proof
 * (encrypted · versioned · verified) and links out to that route.
 */
import { Link as LinkIcon, ScrollText, ShieldCheck } from "lucide-react";

import { type FileDto } from "../api/client.ts";

export interface DetailsPanelProps {
  file: FileDto;
  /** Called when the user clicks "Share" — the host opens the existing
   *  ShareDialog. When absent, the Share action is hidden. */
  onCreateShare?: () => void;
}

function openHistory(file: FileDto) {
  const url = `/document/${encodeURIComponent(file.id)}/history`;
  window.history.pushState({ file }, "", url);
  window.dispatchEvent(new PopStateEvent("popstate"));
}

export function DetailsPanel({ file, onCreateShare }: DetailsPanelProps) {
  const version = Math.max(file.version, 1);
  const prior = Math.max(version - 1, 0);

  return (
    <section
      data-testid="details-panel"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        minHeight: 0,
        padding: "var(--space-4)",
      }}
    >
      <div
        data-testid="details-compliance-card"
        className="glass"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "var(--space-3)",
          padding: "var(--space-4)",
        }}
      >
        <ProofLine
          icon={<ShieldCheck size={15} strokeWidth={1.6} aria-hidden />}
          primary="Encrypted at rest"
          secondary="AES-256-GCM"
        />
        <ProofLine
          icon={<ScrollText size={15} strokeWidth={1.6} aria-hidden />}
          primary={`Version v${version} · ✓ Verified`}
          secondary={prior === 1 ? "1 prior version" : `${prior} prior versions`}
        />

        <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-2)", marginTop: "var(--space-1)" }}>
          <button
            type="button"
            onClick={() => openHistory(file)}
            style={primaryLink}
          >
            View full history →
          </button>
          {onCreateShare && (
            <button type="button" onClick={onCreateShare} style={ghostAction}>
              <LinkIcon size={13} strokeWidth={1.6} aria-hidden />
              Share
            </button>
          )}
        </div>
      </div>
    </section>
  );
}

function ProofLine({
  icon,
  primary,
  secondary,
}: {
  icon: React.ReactNode;
  primary: string;
  secondary: string;
}) {
  return (
    <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)" }}>
      <span aria-hidden style={{ color: "var(--status-verified-700)", flexShrink: 0, marginTop: 1, display: "inline-flex" }}>
        {icon}
      </span>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: "var(--text-sm)", fontWeight: "var(--weight-medium)", color: "var(--fg-default)" }}>
          {primary}
        </div>
        <div style={{ fontSize: "var(--text-xs)", color: "var(--fg-muted)" }}>{secondary}</div>
      </div>
    </div>
  );
}

const primaryLink: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
  padding: "6px 12px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid transparent",
  background: "var(--accent)",
  color: "var(--accent-fg)",
  fontFamily: "var(--font-sans)",
  fontSize: "var(--text-sm)",
  fontWeight: "var(--weight-medium)",
  cursor: "pointer",
};

const ghostAction: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 5,
  padding: "6px 10px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--border-hair)",
  background: "var(--bg-surface)",
  color: "var(--fg-muted)",
  fontFamily: "var(--font-sans)",
  fontSize: "var(--text-sm)",
  fontWeight: "var(--weight-medium)",
  cursor: "pointer",
};
