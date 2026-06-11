/**
 * NT2 — floating formatting toolbar (bubble surface).
 * Spec: docs/research/17-notes-general-user-ux.md §"Floating formatting toolbar".
 *
 * Shows above the user's text selection. Tiptap's `BubbleMenu` extension
 * positions it via floating-ui; the button row is shared with
 * `FixedToolbar` via the [[notes-fixed-toolbar]] plan so both surfaces
 * stay visually + behaviourally in lock-step.
 */
import { BubbleMenu } from "@tiptap/react/menus";
import type { Editor } from "@tiptap/react";

import { ToolbarRow } from "./ToolbarRow.tsx";

interface Props {
  editor: Editor | null;
  /** Opens the link dialog (NT2 Phase 2). Hoisted into the parent so
   * the dialog state can be shared with the mobile + fixed toolbars. */
  onLinkClick: () => void;
}

export function FormattingToolbar({ editor, onLinkClick }: Props) {
  if (!editor) return null;
  return (
    <BubbleMenu
      editor={editor}
      // Hide on empty selection or inside code-block (markdown shortcuts
      // there would be a footgun).
      shouldShow={({ editor, from, to }) => {
        if (from === to) return false;
        if (editor.isActive("codeBlock")) return false;
        return true;
      }}
      options={{
        placement: "top",
        offset: 8,
      }}
    >
      <div className="cd-bubble-toolbar">
        <ToolbarRow
          editor={editor}
          onLinkClick={onLinkClick}
          ariaLabel="Format selection"
        />
      </div>
    </BubbleMenu>
  );
}
