/**
 * VersionHistory — Doc-Hub's flagship compliance surface (UX-18).
 * Spec: docs/ux/18-version-history-surface.md, docs/design/ui-system.md
 * §7.3 (chain timeline), §7.5 (verification badge / tamper alarm).
 *
 * Renders the append-only, hash-chained version timeline for one
 * document, head first. Each node carries `v{n}` (mono/tabular), author,
 * reason, relative time, and the short `content_hash` (click-to-copy).
 * The panel primary is "Verify chain"; a broken chain surfaces a
 * persistent `role="alert"` tamper alarm at the top that cannot be
 * dismissed. Restore is additive ("Restore as new version"). One icon
 * family (Lucide, 1.5px), amber never alone.
 *
 * Two forms from one component:
 *   - variant="panel" — the 360px docked form (DetailsPanel History tab)
 *   - variant="full"  — the full-width route (/document/{id}/history)
 */
import { useCallback, useEffect, useState } from "react";
import {
  Archive,
  Copy,
  Download,
  Gavel,
  GitCommitHorizontal,
  RotateCcw,
  ShieldCheck,
  ShieldOff,
} from "lucide-react";
import { toast } from "sonner";

import {
  ApiError,
  listVersions,
  restoreVersion,
  verifyChain,
  versionContentUrl,
  type FileVersion,
  type VerifyResult,
} from "../api/client.ts";
import { ConfirmDialog } from "./ConfirmDialog.tsx";
import { RegistryMotif } from "./ds/RegistryMotif.tsx";
import { SealBadge } from "./ds/SealBadge.tsx";
import { SkeletonRow } from "./ds/SkeletonRow.tsx";
import { StatusChip } from "./ds/StatusChip.tsx";

const STROKE = 1.5;

/** Per-node step of the travelling verify sweep up the Spine. */
const SPINE_STEP_MS = 90;

/** The Spine's live-verify phase: `idle` (steady), `sweep` (amber travelling
 *  node-to-node UP the chain), `sealed` (settled verified glow). */
type SpinePhase = "idle" | "sweep" | "sealed";

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/** null = not yet verified this session; falls back to the list's
 * `chain_verified`. Otherwise the outcome of an explicit Verify. */
type BadgeState =
  | { kind: "unknown" }
  | { kind: "intact" }
  | { kind: "broken"; atSeq: number };

export function VersionHistory({
  fileId,
  fileName,
  variant = "full",
  onRestored,
}: {
  fileId: string;
  fileName: string;
  variant?: "panel" | "full";
  /** Fired after a successful restore with the new head seq — lets a
   *  host refresh its own file metadata. */
  onRestored?: (newSeq: number) => void;
}) {
  const [versions, setVersions] = useState<FileVersion[] | null>(null);
  const [chainVerified, setChainVerified] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [badge, setBadge] = useState<BadgeState>({ kind: "unknown" });
  const [verifying, setVerifying] = useState(false);
  const [restoreSeq, setRestoreSeq] = useState<number | null>(null);
  // The Spine's travelling-verify animation phase + a monotonic key that
  // remounts the Seal badge to replay its sweep on each successful verify.
  const [spinePhase, setSpinePhase] = useState<SpinePhase>("idle");
  const [sealKey, setSealKey] = useState(0);

  const load = useCallback(async () => {
    setErr(null);
    try {
      const resp = await listVersions(fileId);
      const sorted = [...resp.versions].sort((a, b) => b.seq - a.seq);
      setVersions(sorted);
      setChainVerified(resp.chain_verified);
      setBadge(resp.chain_verified ? { kind: "unknown" } : { kind: "broken", atSeq: resp.head_seq });
      setSpinePhase("idle");
    } catch (e) {
      setErr((e as ApiError).message ?? "Couldn't load version history.");
      setVersions([]);
    }
  }, [fileId]);

  useEffect(() => {
    void load();
  }, [load]);

  async function runVerify() {
    if (verifying) return;
    setVerifying(true);
    try {
      const r: VerifyResult = await verifyChain(fileId);
      if (r.status === "intact") {
        setBadge({ kind: "intact" });
        setChainVerified(true);
        setSealKey((k) => k + 1);
        // Travel the amber up the Spine, then settle into the verified glow.
        // Reduced-motion jumps straight to the settled state (no travel).
        const nodes = versions?.length ?? 0;
        if (prefersReducedMotion() || nodes === 0) {
          setSpinePhase("sealed");
        } else {
          setSpinePhase("sweep");
          window.setTimeout(
            () => setSpinePhase("sealed"),
            nodes * SPINE_STEP_MS + 420,
          );
        }
        toast.success("Chain verified · every link intact");
      } else {
        setBadge({ kind: "broken", atSeq: r.at_seq });
        setChainVerified(false);
        setSpinePhase("idle");
      }
    } catch (e) {
      toast.error((e as ApiError).message ?? "Verification failed.");
    } finally {
      setVerifying(false);
    }
  }

  async function doRestore(seq: number) {
    const updated = await restoreVersion(fileId, seq);
    toast.success(`Version ${updated.version} saved · restored from v${seq}`);
    await load();
    onRestored?.(updated.version);
  }

  // Resolved tamper state — an explicit broken badge, or a list that
  // arrived already flagged. Persistent; drives the top-of-panel alarm.
  const broken =
    badge.kind === "broken"
      ? badge.atSeq
      : !chainVerified && versions && versions.length > 0
        ? versions[versions.length - 1].seq
        : null;

  return (
    <section
      aria-label={`Version history for ${fileName}`}
      style={{ display: "flex", flexDirection: "column", minHeight: 0, height: "100%" }}
    >
      <Header
        fileName={fileName}
        variant={variant}
        badge={broken != null ? { kind: "broken", atSeq: broken } : badge}
        verifying={verifying}
        onVerify={runVerify}
        canVerify={!!versions && versions.length > 0}
        sealKey={sealKey}
      />

      {broken != null && <TamperAlarm atSeq={broken} />}

      <div style={{ flex: 1, minHeight: 0, overflowY: "auto" }}>
        {err && (
          <div role="alert" style={errBox}>
            {err}
          </div>
        )}

        {versions === null ? (
          <LoadingTimeline />
        ) : versions.length === 0 && !err ? (
          <EmptyChain />
        ) : versions.length === 1 ? (
          <>
            <Timeline
              versions={versions}
              fileId={fileId}
              broken={broken}
              phase={spinePhase}
              onRestore={setRestoreSeq}
            />
            <OneVersion />
          </>
        ) : (
          <Timeline
            versions={versions}
            fileId={fileId}
            broken={broken}
            phase={spinePhase}
            onRestore={setRestoreSeq}
          />
        )}
      </div>

      {versions && versions.length > 0 && (
        <Footer count={versions.length} verified={broken == null && chainVerified} />
      )}

      <ConfirmDialog
        open={restoreSeq !== null}
        title={restoreSeq !== null ? `Restore v${restoreSeq} as a new version?` : ""}
        body={
          restoreSeq !== null
            ? `This appends a new head, byte-identical to v${restoreSeq}. The current version and all prior versions are kept — nothing is destroyed.`
            : undefined
        }
        confirmLabel="Restore"
        onConfirm={async () => {
          if (restoreSeq === null) return;
          try {
            await doRestore(restoreSeq);
          } catch (e) {
            toast.error((e as ApiError).message ?? "Restore failed.");
            throw e; // keep the dialog open for retry
          }
        }}
        onClose={() => setRestoreSeq(null)}
      />
    </section>
  );
}

// ── Header ─────────────────────────────────────────────────────────────

function Header({
  fileName,
  variant,
  badge,
  verifying,
  onVerify,
  canVerify,
  sealKey,
}: {
  fileName: string;
  variant: "panel" | "full";
  badge: BadgeState;
  verifying: boolean;
  onVerify: () => void;
  canVerify: boolean;
  /** Bumped on each successful verify to replay the Seal sweep. */
  sealKey: number;
}) {
  return (
    <header
      className="glass--thin"
      style={{
        flex: "0 0 auto",
        display: "flex",
        alignItems: "center",
        gap: "var(--space-3)",
        padding: "var(--space-3)",
        marginBottom: "var(--space-2)",
      }}
    >
      <div style={{ minWidth: 0, flex: 1 }}>
        {variant === "full" && (
          <div className="caps-label" style={{ marginBottom: 4 }}>
            Version history
          </div>
        )}
        <div
          style={{
            fontSize: variant === "full" ? "var(--text-lg)" : "var(--text-md)",
            fontWeight: "var(--weight-semibold)",
            color: "var(--fg-default)",
            letterSpacing: "var(--tracking-tight)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {fileName}
        </div>
        <div style={{ marginTop: 3 }}>
          <Badge badge={badge} sealKey={sealKey} />
        </div>
      </div>
      <button
        type="button"
        onClick={onVerify}
        disabled={!canVerify || verifying}
        style={verifyBtn(!canVerify || verifying)}
      >
        <ShieldCheck size={14} strokeWidth={STROKE} aria-hidden />
        {verifying ? "Verifying…" : "Verify chain"}
      </button>
    </header>
  );
}

/** The two-variant verification badge (§7.5) — icon + label, never
 *  colour alone. `unknown` reads as the calm verified default; `intact`
 *  (a fresh explicit verify) plays the Seal moment (§6). */
function Badge({ badge, sealKey }: { badge: BadgeState; sealKey: number }) {
  if (badge.kind === "broken") {
    return (
      <StatusChip
        tone="danger"
        icon={<ShieldOff size={13} strokeWidth={STROKE} />}
        label="Tamper detected"
        title={`Tamper detected — chain verification failed at v${badge.atSeq}`}
      />
    );
  }
  if (badge.kind === "intact") {
    // The Seal — an amber specular sweep settling into the mono caption.
    // Remounted by `sealKey` so each successful verify replays it.
    return (
      <SealBadge
        key={sealKey}
        caption="Verified · SHA-256 · sealed"
        title="Chain intact — every link verified"
      />
    );
  }
  return (
    <StatusChip
      tone="verified"
      icon={<ShieldCheck size={13} strokeWidth={STROKE} />}
      label="Verified"
      title="Chain intact — every link verified"
    />
  );
}

// ── Tamper alarm (§8.2 / principle 9) ──────────────────────────────────

function TamperAlarm({ atSeq }: { atSeq: number }) {
  return (
    <div
      role="alert"
      aria-live="assertive"
      style={{
        display: "flex",
        gap: "var(--space-2)",
        alignItems: "flex-start",
        padding: "var(--space-3)",
        margin: "var(--space-2) 0",
        background: "var(--amber-tint)",
        borderRadius: "var(--radius-md)",
        borderLeft: "3px solid var(--amber-700)",
        boxShadow: "var(--accent-glow)",
        color: "var(--fg-default)",
        fontSize: "var(--text-sm)",
        lineHeight: "var(--leading-sm)",
      }}
    >
      <span aria-hidden style={{ color: "var(--amber-700)", flexShrink: 0, marginTop: 1 }}>
        <ShieldOff size={16} strokeWidth={STROKE} />
      </span>
      <div>
        <div style={{ fontWeight: "var(--weight-semibold)", color: "var(--amber-700)" }}>
          Tamper detected · chain verification failed at v{atSeq}
        </div>
        <div style={{ marginTop: 2, color: "var(--fg-muted)" }}>
          The stored bytes no longer match this version's recorded hash. Reported to admins. This
          cannot be dismissed until resolved.
        </div>
      </div>
    </div>
  );
}

// ── Timeline ───────────────────────────────────────────────────────────

/** The Spine (ui-vision-2026 §5.4) — the hash chain rendered as a living
 *  vertical ledger: a connective amber-capable axis, glass version nodes,
 *  the current version emphasised, mono hash pills. On Verify the amber
 *  travels node-to-node UP the spine (`phase="sweep"`, staggered by reverse
 *  index) and settles into a verified glow (`phase="sealed"`). */
function Timeline({
  versions,
  fileId,
  broken,
  phase,
  onRestore,
}: {
  versions: FileVersion[];
  fileId: string;
  broken: number | null;
  phase: SpinePhase;
  onRestore: (seq: number) => void;
}) {
  const count = versions.length;
  return (
    <ul className="cd-spine" data-phase={phase} style={{ listStyle: "none", margin: 0, padding: "var(--space-2) 0 0" }}>
      <SpineStyle />
      {versions.map((v, i) => {
        const isHead = i === 0;
        const isLast = i === count - 1;
        // The connector below a node carries the link to its predecessor
        // (the next, lower-seq node). It breaks at the reported seq.
        const linkBroken = broken != null && v.seq === broken;
        // Reverse index: the oldest (bottom) node is 0 so the verify sweep
        // lights it first and travels UP, arriving at the head last.
        const rev = count - 1 - i;
        return (
          <VersionNode
            key={v.seq}
            v={v}
            fileId={fileId}
            isHead={isHead}
            isLast={isLast}
            linkBroken={linkBroken}
            rev={rev}
            onRestore={onRestore}
          />
        );
      })}
    </ul>
  );
}

/** Self-contained Spine motion + finish — a real connective axis that the
 *  verify sweep travels up, then settles to a verified glow. Only opacity /
 *  transform / colour animate; reduced-motion drops the travel. */
function SpineStyle() {
  return (
    <style>{`
      .cd-spine-dot {
        color: var(--fg-subtle);
        display: inline-flex;
        margin-top: 2px;
        transition: color var(--dur-base) var(--ease-out),
                    filter var(--dur-base) var(--ease-out),
                    transform var(--dur-base) var(--ease-out);
      }
      .cd-spine-dot[data-head="true"] { color: var(--accent); }
      .cd-spine-link {
        flex: 1;
        width: 2px;
        min-height: 18px;
        margin: 2px 0;
        background: var(--border-hair);
        transition: background var(--dur-base) var(--ease-out),
                    box-shadow var(--dur-base) var(--ease-out);
      }
      .cd-spine-link[data-broken="true"] {
        width: 0;
        background: transparent;
        border-left: 2px dashed var(--amber-700);
        box-shadow: none !important;
      }
      /* Settled verified glow along the whole chain. */
      .cd-spine[data-phase="sealed"] .cd-spine-dot {
        color: var(--accent);
        filter: drop-shadow(0 0 4px var(--amber-glow-1));
      }
      .cd-spine[data-phase="sealed"] .cd-spine-link:not([data-broken="true"]) {
        background: var(--amber-glow-1);
        box-shadow: 0 0 6px var(--amber-glow-2);
      }
      /* The travelling sweep — each node lights in turn, bottom → head. */
      .cd-spine[data-phase="sweep"] .cd-spine-dot {
        animation: cd-spine-dot-lit var(--dur-base) var(--ease-seal) both;
        animation-delay: calc(var(--rev, 0) * ${SPINE_STEP_MS}ms);
      }
      .cd-spine[data-phase="sweep"] .cd-spine-link:not([data-broken="true"]) {
        animation: cd-spine-link-lit var(--dur-base) var(--ease-out) both;
        animation-delay: calc(var(--rev, 0) * ${SPINE_STEP_MS}ms);
      }
      @keyframes cd-spine-dot-lit {
        0%   { color: var(--fg-subtle); transform: scale(1);
               filter: drop-shadow(0 0 0 transparent); }
        55%  { color: var(--accent); transform: scale(1.35);
               filter: drop-shadow(0 0 7px var(--amber-glow-1)); }
        100% { color: var(--accent); transform: scale(1);
               filter: drop-shadow(0 0 4px var(--amber-glow-1)); }
      }
      @keyframes cd-spine-link-lit {
        0%   { background: var(--border-hair); box-shadow: none; }
        100% { background: var(--amber-glow-1); box-shadow: 0 0 6px var(--amber-glow-2); }
      }
      @media (prefers-reduced-motion: reduce) {
        .cd-spine[data-phase="sweep"] .cd-spine-dot,
        .cd-spine[data-phase="sweep"] .cd-spine-link { animation: none; }
      }
    `}</style>
  );
}

function VersionNode({
  v,
  fileId,
  isHead,
  isLast,
  linkBroken,
  rev,
  onRestore,
}: {
  v: FileVersion;
  fileId: string;
  isHead: boolean;
  isLast: boolean;
  linkBroken: boolean;
  rev: number;
  onRestore: (seq: number) => void;
}) {
  const author = v.author?.name ?? v.author_name ?? "—";
  return (
    <li style={{ display: "flex", gap: "var(--space-3)", ["--rev" as string]: rev }}>
      {/* Rail — node dot + connector (the connector IS the chain). */}
      <div
        aria-hidden
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          flexShrink: 0,
          width: 20,
        }}
      >
        <span className="cd-spine-dot" data-head={isHead ? "true" : undefined}>
          <GitCommitHorizontal
            size={18}
            strokeWidth={STROKE}
            fill={isHead ? "currentColor" : "none"}
          />
        </span>
        {!isLast && (
          <span className="cd-spine-link" data-broken={linkBroken ? "true" : undefined} />
        )}
      </div>

      {/* Body — a near-solid glass node card. */}
      <div
        className="glass--thick"
        style={{ flex: 1, minWidth: 0, padding: "var(--space-3)", marginBottom: "var(--space-3)" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <span
            className="mono"
            style={{ fontSize: "var(--mono-sm)", color: "var(--fg-muted)", fontWeight: "var(--weight-medium)" }}
          >
            v{v.seq}
          </span>
          {isHead && (
            <span className="caps-label" style={{ color: "var(--accent-press)" }}>
              current
            </span>
          )}
          {v.held && (
            <StatusChip
              tone="attention"
              icon={<Gavel size={12} strokeWidth={STROKE} />}
              label="hold"
              title="Under an active legal hold"
            />
          )}
          {v.tombstoned && (
            <StatusChip
              tone="ambient"
              icon={<Archive size={12} strokeWidth={STROKE} />}
              label="retained"
              title="Tombstoned — bytes retained, never hidden"
            />
          )}
          <span style={{ flex: 1 }} />
          <span
            className="tnum"
            title={new Date(v.created_at).toLocaleString()}
            style={{ fontSize: "var(--text-xs)", color: "var(--fg-subtle)", whiteSpace: "nowrap" }}
          >
            {relTime(v.created_at)}
          </span>
        </div>

        <div style={{ marginTop: 2, fontSize: "var(--text-xs)", color: "var(--fg-subtle)" }}>
          {author}
          {v.size > 0 && <> · {formatBytes(v.size)}</>}
        </div>

        {v.reason && (
          <div
            style={{
              marginTop: 4,
              fontSize: "var(--text-sm)",
              color: "var(--fg-muted)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {v.reason}
          </div>
        )}

        <div style={{ marginTop: 6, display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <HashChip label="hash" hash={v.content_hash} />
          {v.prev_hash && <HashChip label="prev" hash={v.prev_hash} muted />}
          <span style={{ flex: 1 }} />
          <a
            href={versionContentUrl(fileId, v.seq)}
            download={`v${v.seq}-${downloadName(v)}`}
            style={ghostAction}
            title={`Download v${v.seq}`}
          >
            <Download size={13} strokeWidth={STROKE} aria-hidden />
            Download
          </a>
          {!isHead && (
            <button
              type="button"
              onClick={() => onRestore(v.seq)}
              style={ghostAction}
              title={`Restore v${v.seq} as a new version`}
            >
              <RotateCcw size={13} strokeWidth={STROKE} aria-hidden />
              Restore
            </button>
          )}
        </div>
      </div>
    </li>
  );
}

/** Truncated, click-to-copy hash. Full value lives in title + aria-label
 *  (§9.5) — never only the truncated visual. */
function HashChip({ label, hash, muted }: { label: string; hash: string; muted?: boolean }) {
  return (
    <button
      type="button"
      onClick={() => {
        void navigator.clipboard?.writeText(hash);
        toast.success("Hash copied");
      }}
      aria-label={`${label} ${hash} — copy`}
      title={`${label}: ${hash}`}
      className="mono"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        padding: "1px 6px",
        background: "var(--bg-sunken)",
        border: "1px solid var(--border-hair)",
        borderRadius: "var(--radius-xs)",
        fontSize: "var(--mono-xs)",
        color: muted ? "var(--fg-subtle)" : "var(--fg-muted)",
        cursor: "pointer",
      }}
    >
      {label !== "hash" && <span style={{ color: "var(--fg-subtle)" }}>{label}</span>}
      {shortHash(hash)}
      <Copy size={11} strokeWidth={STROKE} aria-hidden style={{ opacity: 0.6 }} />
    </button>
  );
}

// ── States ─────────────────────────────────────────────────────────────

function LoadingTimeline() {
  return (
    <div style={{ paddingTop: "var(--space-2)" }} aria-busy="true" aria-label="Loading versions">
      {Array.from({ length: 3 }).map((_, i) => (
        <SkeletonRow key={i} columns={4} />
      ))}
    </div>
  );
}

function OneVersion() {
  return (
    <div className="glass" style={emptyBox}>
      <RegistryMotif overlay="layers" />
      <div style={{ fontSize: "var(--text-sm)", fontWeight: "var(--weight-medium)", color: "var(--fg-default)" }}>
        One version so far
      </div>
      <div style={{ fontSize: "var(--text-xs)", color: "var(--fg-muted)", maxWidth: 280 }}>
        History begins here. Every save appends a new, hash-chained version you can verify, restore,
        and download.
      </div>
    </div>
  );
}

function EmptyChain() {
  return (
    <div className="glass" style={emptyBox}>
      <RegistryMotif overlay="scroll-text" />
      <div style={{ fontSize: "var(--text-sm)", fontWeight: "var(--weight-medium)", color: "var(--fg-default)" }}>
        No versions yet
      </div>
      <div style={{ fontSize: "var(--text-xs)", color: "var(--fg-muted)", maxWidth: 280 }}>
        This document's version chain is empty. It fills in as the file is saved.
      </div>
    </div>
  );
}

function Footer({ count, verified }: { count: number; verified: boolean }) {
  return (
    <footer
      className="glass--thin"
      style={{
        flex: "0 0 auto",
        display: "flex",
        alignItems: "center",
        gap: "var(--space-2)",
        padding: "var(--space-2) var(--space-3)",
        marginTop: "var(--space-1)",
        fontSize: "var(--text-xs)",
        color: "var(--fg-subtle)",
      }}
    >
      <span className="tnum">{count}</span>
      <span>{count === 1 ? "version" : "versions"}</span>
      <span>·</span>
      {verified ? (
        <StatusChip
          tone="verified"
          icon={<ShieldCheck size={12} strokeWidth={STROKE} />}
          label="chain intact"
        />
      ) : (
        <StatusChip
          tone="danger"
          icon={<ShieldOff size={12} strokeWidth={STROKE} />}
          label="chain broken"
        />
      )}
      <span style={{ flex: 1 }} />
      <span>Append-only · hash-chained</span>
    </footer>
  );
}

// ── helpers ────────────────────────────────────────────────────────────

const errBox: React.CSSProperties = {
  margin: "var(--space-2) 0",
  padding: "var(--space-2) var(--space-3)",
  background: "var(--amber-tint)",
  borderLeft: "3px solid var(--status-danger)",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--text-sm)",
  color: "var(--fg-default)",
};

const emptyBox: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  justifyContent: "center",
  gap: "var(--space-2)",
  padding: "var(--space-8) var(--space-4)",
  textAlign: "center",
};

function verifyBtn(disabled: boolean): React.CSSProperties {
  return {
    display: "inline-flex",
    alignItems: "center",
    gap: 6,
    flexShrink: 0,
    padding: "6px 12px",
    borderRadius: "var(--radius-sm)",
    border: "1px solid transparent",
    background: "var(--accent)",
    color: "var(--accent-fg)",
    fontFamily: "var(--font-sans)",
    fontSize: "var(--text-sm)",
    fontWeight: "var(--weight-medium)",
    cursor: disabled ? "default" : "pointer",
    opacity: disabled ? 0.55 : 1,
  };
}

const ghostAction: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 5,
  padding: "3px 8px",
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--border-hair)",
  background: "var(--bg-surface)",
  color: "var(--fg-muted)",
  fontFamily: "var(--font-sans)",
  fontSize: "var(--text-xs)",
  fontWeight: "var(--weight-medium)",
  textDecoration: "none",
  cursor: "pointer",
};

function shortHash(h: string): string {
  if (h.length <= 8) return h;
  return `${h.slice(0, 4)}…${h.slice(-2)}`;
}

function downloadName(v: FileVersion): string {
  return v.content_hash ? `${v.content_hash.slice(0, 8)}` : `seq${v.seq}`;
}

function relTime(iso: string): string {
  const then = new Date(iso).getTime();
  const diff = Date.now() - then;
  const abs = Math.abs(diff);
  if (abs < 60_000) return "just now";
  if (abs < 3_600_000) return `${Math.round(abs / 60_000)}m ago`;
  if (abs < 86_400_000) return `${Math.round(abs / 3_600_000)}h ago`;
  if (abs < 7 * 86_400_000) return `${Math.round(abs / 86_400_000)}d ago`;
  return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function formatBytes(b: number): string {
  if (b === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${i === 0 ? v : v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}
