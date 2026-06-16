/**
 * FilePresenceStack — avatar pile for the `<FileFullscreen>` editor
 * header. Shows every peer (not self) currently viewing this file.
 *
 * Sibling of `<AvatarStack>` (which shows everyone online in the
 * workspace). The shape + tint colours match so the two stacks read
 * as part of the same design language; the only difference is this
 * one filters by `viewing === fileId` and renders a tighter 5-pile
 * with a "+N" overflow chip.
 *
 * Collapses to nothing when no peers are viewing the file — no
 * layout shift when peers arrive (the header reserves the slot).
 */
import { usePeersViewingFile, type PresenceUser } from "../state/PresenceContext.tsx";

const MAX_VISIBLE = 4;
const SIZE = 26;
const OVERLAP = 8;

export function FilePresenceStack({ fileId }: { fileId: string | null | undefined }) {
  const peers = usePeersViewingFile(fileId);
  if (peers.length === 0) return null;

  const visible = peers.slice(0, MAX_VISIBLE);
  const overflow = peers.length - visible.length;

  return (
    <div
      role="group"
      data-testid="file-fullscreen-presence"
      aria-label={`${peers.length} other ${peers.length === 1 ? "person" : "people"} viewing this file`}
      style={{ display: "flex", alignItems: "center" }}
    >
      <div style={{ display: "flex", alignItems: "center" }}>
        {visible.map((u, i) => (
          <Avatar key={u.user_id} user={u} stackIndex={i} />
        ))}
      </div>
      {overflow > 0 && (
        <span
          title={`${overflow} more`}
          aria-label={`${overflow} more`}
          style={{
            marginLeft: -OVERLAP + 4,
            fontSize: 11,
            fontWeight: 600,
            color: "var(--muted)",
            background: "var(--card)",
            border: "1px solid var(--line)",
            borderRadius: SIZE / 2,
            height: SIZE,
            minWidth: SIZE,
            padding: "0 7px",
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          +{overflow}
        </span>
      )}
    </div>
  );
}

function Avatar({ user, stackIndex }: { user: PresenceUser; stackIndex: number }) {
  const initials = monogramOf(user.username);
  return (
    <span
      title={`${user.username} viewing`}
      aria-label={user.username}
      style={{
        width: SIZE,
        height: SIZE,
        borderRadius: "50%",
        background: user.tint,
        color: "var(--paper)",
        fontSize: 11,
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: 0.2,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        border: "2px solid var(--card)",
        boxShadow: "0 1px 2px rgba(0,0,0,0.08)",
        marginLeft: stackIndex === 0 ? 0 : -OVERLAP,
        zIndex: 10 - stackIndex,
        flexShrink: 0,
      }}
    >
      {initials}
    </span>
  );
}

function monogramOf(name: string): string {
  const parts = name.trim().split(/\s+/);
  if (parts.length === 0 || parts[0] === "") return "?";
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  return parts[0].slice(0, 2).toUpperCase();
}
