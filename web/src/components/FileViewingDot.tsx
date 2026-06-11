/**
 * RT3 — file-row "someone else is viewing" indicator.
 *
 * Spec: docs/research/14-presence.md §"SPA surface" — 4 px tinted
 * dot in the corner of any file row that has another user viewing
 * it. Hover → "Alex".
 *
 * Quiet by default — renders `null` when no peer is viewing the
 * file. Layout is unaffected (positioned absolutely on top of the
 * card / row).
 *
 * Source of truth: `useViewingFile(fileId)` from PresenceContext.
 * The current user is already self-excluded inside the hook, so the
 * dot only ever surfaces OTHER peers.
 */
import { useViewingFile } from "../state/PresenceContext.tsx";

interface Props {
  fileId: string;
  /** Where to place the dot inside the parent (which must be
   * `position: relative`). The card variant tucks it into the
   * top-left of the thumbnail; the list variant rides just left of
   * the row's filename label. */
  placement?: "card" | "list";
}

export function FileViewingDot({ fileId, placement = "card" }: Props) {
  const peer = useViewingFile(fileId);
  if (!peer) return null;

  const size = 7;
  const ring = 2;

  const positional: React.CSSProperties =
    placement === "card"
      ? { position: "absolute", top: 10, left: 10, zIndex: 4 }
      : { display: "inline-block", marginRight: 6, verticalAlign: "middle" };

  return (
    <span
      role="img"
      aria-label={`${peer.username} is viewing this file`}
      title={`${peer.username} is viewing`}
      style={{
        ...positional,
        width: size,
        height: size,
        borderRadius: "50%",
        background: peer.tint,
        boxShadow: `0 0 0 ${ring}px var(--paper)`,
        pointerEvents: "auto",
      }}
    />
  );
}
