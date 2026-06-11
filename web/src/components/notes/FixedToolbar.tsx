/**
 * Notes fixed (sticky-top) formatting toolbar.
 *
 * Spec: [[notes-fixed-toolbar]] memory entry.
 *
 * Pinned to the top of the editor's scrolling region. Always visible
 * on desktop; hidden on mobile (≤1023 px) because NT6's bottom
 * toolbar covers that surface. Same button row as the bubble menu —
 * see `ToolbarRow.tsx`.
 */
import type { Editor } from "@tiptap/react";

import { ToolbarRow } from "./ToolbarRow.tsx";

interface Props {
  editor: Editor | null;
  onLinkClick: () => void;
}

export function FixedToolbar({ editor, onLinkClick }: Props) {
  if (!editor) return null;
  return (
    <div className="cd-fixed-toolbar">
      <ToolbarRow editor={editor} onLinkClick={onLinkClick} ariaLabel="Format" />
    </div>
  );
}
