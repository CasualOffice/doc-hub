/**
 * Shared formatting-toolbar row. Two surfaces consume it:
 *   - `FormattingToolbar` (bubble, on text selection)
 *   - `FixedToolbar`     (sticky-top, always visible on desktop)
 *
 * Spec: [[notes-fixed-toolbar]] memory + docs/research/17-notes-general-user-ux.md.
 * Both surfaces render the same buttons with the same `editor.isActive`
 * driven state — keeps "what does Bold look like right now" consistent
 * regardless of which surface the user reached for.
 */
import type { Editor } from "@tiptap/react";
import {
  Bold as BoldIcon,
  Italic as ItalicIcon,
  Strikethrough,
  Code as CodeIcon,
  Link as LinkIcon,
  Quote,
  List,
  ListOrdered,
  Heading1,
  Heading2,
  Heading3,
} from "lucide-react";

export interface ToolbarRowProps {
  editor: Editor;
  onLinkClick: () => void;
  /** Used as the row's `aria-label` — bubble surface says "Format
   * selection" while the fixed surface says "Format". */
  ariaLabel: string;
  /** Optional override class so each surface can theme its own
   * button + separator (e.g. bubble has a darker tint than fixed). */
  buttonClass?: string;
  sepClass?: string;
}

export function ToolbarRow({
  editor,
  onLinkClick,
  ariaLabel,
  buttonClass = "cd-bubble-btn",
  sepClass = "cd-bubble-sep",
}: ToolbarRowProps) {
  return (
    <div role="toolbar" aria-label={ariaLabel} className="cd-toolbar-row">
      <Btn
        cls={buttonClass}
        label="Bold"
        shortcut="⌘B"
        active={editor.isActive("bold")}
        onClick={() => editor.chain().focus().toggleBold().run()}
      >
        <BoldIcon size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Italic"
        shortcut="⌘I"
        active={editor.isActive("italic")}
        onClick={() => editor.chain().focus().toggleItalic().run()}
      >
        <ItalicIcon size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Strikethrough"
        active={editor.isActive("strike")}
        onClick={() => editor.chain().focus().toggleStrike().run()}
      >
        <Strikethrough size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Inline code"
        active={editor.isActive("code")}
        onClick={() => editor.chain().focus().toggleCode().run()}
      >
        <CodeIcon size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label={editor.isActive("link") ? "Edit link" : "Add link"}
        shortcut="⌘K"
        active={editor.isActive("link")}
        onClick={onLinkClick}
      >
        <LinkIcon size={14} strokeWidth={2} />
      </Btn>
      <Sep cls={sepClass} />
      <Btn
        cls={buttonClass}
        label="Heading 1"
        active={editor.isActive("heading", { level: 1 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
      >
        <Heading1 size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Heading 2"
        active={editor.isActive("heading", { level: 2 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
      >
        <Heading2 size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Heading 3"
        active={editor.isActive("heading", { level: 3 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
      >
        <Heading3 size={14} strokeWidth={2} />
      </Btn>
      <Sep cls={sepClass} />
      <Btn
        cls={buttonClass}
        label="Bullet list"
        active={editor.isActive("bulletList")}
        onClick={() => editor.chain().focus().toggleBulletList().run()}
      >
        <List size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Numbered list"
        active={editor.isActive("orderedList")}
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
      >
        <ListOrdered size={14} strokeWidth={2} />
      </Btn>
      <Btn
        cls={buttonClass}
        label="Blockquote"
        active={editor.isActive("blockquote")}
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
      >
        <Quote size={14} strokeWidth={2} />
      </Btn>
    </div>
  );
}

function Btn({
  cls,
  label,
  shortcut,
  active,
  onClick,
  children,
}: {
  cls: string;
  label: string;
  shortcut?: string;
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={shortcut ? `${label} (${shortcut})` : label}
      aria-pressed={active}
      title={shortcut ? `${label} · ${shortcut}` : label}
      onMouseDown={(e) => {
        // Prevent the selection from collapsing before the command runs.
        e.preventDefault();
      }}
      onClick={onClick}
      className={`${cls}${active ? " is-active" : ""}`}
    >
      {children}
    </button>
  );
}

function Sep({ cls }: { cls: string }) {
  return <span aria-hidden="true" className={cls} />;
}
