/**
 * Phase 3 §17 — live-render markdown editor.
 * Spec: docs/research/17-notes-general-user-ux.md.
 *
 * Foundation (Phase 1) — what this delivers today:
 *   - Single pane. The editor IS the document.
 *   - Markdown body parses in; markdown body comes out.
 *   - Live-render: typing `# `, `**bold**`, `- item`, `> quote`, `\`code\``
 *     etc. collapses syntax into formatted blocks the moment the
 *     pattern is unambiguous. No source ever visible.
 *   - Existing notes load without migration (markdown stays the
 *     storage format).
 *
 * Deferred (Phase 2+, separate PIPELINE rows):
 *   - Slash menu (NT3).
 *   - Floating formatting toolbar on selection (NT2).
 *   - `@` / `+` / `[[` pickers (NT4).
 *   - Hover-revealed drag handles (NT5).
 *   - Mobile sticky toolbar (NT6).
 *
 * AI integration seam — the slash menu's command list is where
 * `/ask AI`, summarise, translate would land. Path-only, not work,
 * per PIPELINE §"Path-only AI integration seams".
 */
import { useEffect, useRef } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "tiptap-markdown";

import { listWorkspaceMembers, type NoteNode } from "../../api/client.ts";

import { BlockHandle } from "./BlockHandle.tsx";
import { FormattingToolbar } from "./FormattingToolbar.tsx";
import { MobileToolbar } from "./MobileToolbar.tsx";
import { slashMenuExtension } from "./slashMenu.ts";
import { SlashMenuPopover, type SlashPopoverHandle } from "./SlashMenuPopover.tsx";
import { peopleMentionExtension } from "./peopleMention.ts";
import { MentionPopover, type MentionPopoverHandle } from "./MentionPopover.tsx";
import { noteLinkExtension } from "./noteLink.ts";
import { NoteLinkPopover, type NoteLinkPopoverHandle } from "./NoteLinkPopover.tsx";

interface Props {
  /** Markdown source. Loaded on mount; subsequent prop changes (e.g.
   * note switch) refresh the editor. */
  value: string;
  /** Fires on every keystroke with the serialized markdown. The
   * parent debounces + persists. */
  onChange: (markdown: string) => void;
  /** Disables editing while a save is in flight or the SPA's offline. */
  readOnly?: boolean;
  /** Placeholder text — shown only when the document is empty. */
  placeholder?: string;
  /** Optional id for accessibility wiring. */
  id?: string;
  /** Active workspace id — drives the `@` mention picker's member
   * fetch. When unset, the picker stays empty (degrades cleanly). */
  workspaceId?: string | null;
  /** Notes in the active workspace — drives the `+` note-link
   * picker. The parent already holds this list for the tree view; we
   * pass it through as a read-only reference. */
  notesTree?: NoteNode[];
  /** Called when the user picks "Create page «query»" in the `+`
   * popover. Parent creates the note + the wiki-link inserts the
   * title verbatim. */
  onCreateNote?: (title: string) => void;
}

export function MarkdownEditor({
  value,
  onChange,
  readOnly,
  placeholder,
  id,
  workspaceId,
  notesTree,
  onCreateNote,
}: Props) {
  // Track the markdown the editor is currently "synced to" so prop
  // updates from autosave don't fight live edits.
  const lastEmittedRef = useRef(value);
  // Bridges between the Tiptap suggestion plugins and their React
  // popovers. Each extension calls into the matching handle on every
  // keystroke + selection move.
  const slashPopoverRef = useRef<SlashPopoverHandle | null>(null);
  const mentionPopoverRef = useRef<MentionPopoverHandle | null>(null);
  const noteLinkPopoverRef = useRef<NoteLinkPopoverHandle | null>(null);
  // Live refs for the workspace + tree so the extensions see the
  // current values without forcing the editor to remount on every
  // tree refresh.
  const workspaceIdRef = useRef(workspaceId ?? null);
  const notesTreeRef = useRef<NoteNode[]>(notesTree ?? []);
  useEffect(() => {
    workspaceIdRef.current = workspaceId ?? null;
  }, [workspaceId]);
  useEffect(() => {
    notesTreeRef.current = notesTree ?? [];
  }, [notesTree]);

  const editor = useEditor({
    extensions: [
      // StarterKit ships paragraph, heading, bold, italic, strike,
      // code, blockquote, bulletList, orderedList, listItem,
      // codeBlock, hardBreak, horizontalRule, history, dropcursor,
      // gapcursor — and the markdown input-rules + keyboard shortcuts
      // for each. This IS the live-render behaviour.
      StarterKit.configure({
        // We use `tiptap-markdown` for serialization, not Tiptap's
        // own HTML mode.
        codeBlock: { HTMLAttributes: { spellcheck: "false" } },
        heading: { levels: [1, 2, 3] },
      }),
      // Markdown round-trip. `transformPastedText` lets users paste
      // raw markdown and have it format inline. `linkify` auto-detects
      // URLs typed inline.
      Markdown.configure({
        html: false,
        tightLists: true,
        linkify: true,
        transformPastedText: true,
      }),
      // NT3 slash menu. The extension owns the trigger detection +
      // filtering; the popover owns the UI. The handle bridges them.
      slashMenuExtension({
        onUpdate: (state) => slashPopoverRef.current?.update(state),
        onExit: () => slashPopoverRef.current?.hide(),
        onKeyDown: (e) => slashPopoverRef.current?.onKeyDown(e) ?? false,
      }),
      // NT4 `@` people-mention picker. Members are fetched once per
      // workspace per editor instance — cache lives inside the
      // extension. `[[` parity ships in NT4 Phase 2.
      peopleMentionExtension({
        loadMembers: async () => {
          const ws = workspaceIdRef.current;
          if (!ws) return [];
          const r = await listWorkspaceMembers(ws);
          return r.members;
        },
        controls: {
          onUpdate: (state) => mentionPopoverRef.current?.update(state),
          onExit: () => mentionPopoverRef.current?.hide(),
          onKeyDown: (e) => mentionPopoverRef.current?.onKeyDown(e) ?? false,
        },
      }),
      // NT4 `+` note-link picker. Tree is read from a ref so the
      // extension always sees the latest list.
      noteLinkExtension({
        loadNotes: () => notesTreeRef.current,
        controls: {
          onUpdate: (state) => noteLinkPopoverRef.current?.update(state),
          onExit: () => noteLinkPopoverRef.current?.hide(),
          onKeyDown: (e) => noteLinkPopoverRef.current?.onKeyDown(e) ?? false,
        },
      }),
    ],
    content: value,
    editable: !readOnly,
    immediatelyRender: false, // hydrates after mount; safer with React 19 strict mode
    onUpdate: ({ editor }) => {
      const md = getMarkdown(editor);
      lastEmittedRef.current = md;
      onChange(md);
    },
    editorProps: {
      attributes: {
        // Token-driven styling from globals.css; the editor inherits
        // the rest of the type stack. Spellcheck off in code blocks
        // (set above); on elsewhere.
        class: "cd-note-editor",
        // Native `role=textbox` plus our id for label association.
        ...(id ? { id } : {}),
        ...(placeholder ? { "data-placeholder": placeholder } : {}),
      },
    },
  });

  // Sync external value changes (e.g. user switched notes) without
  // wiping out unsaved keystrokes. `setContent` would otherwise lose
  // cursor position + history.
  useEffect(() => {
    if (!editor) return;
    if (value === lastEmittedRef.current) return;
    editor.commands.setContent(value, { emitUpdate: false });
    lastEmittedRef.current = value;
  }, [editor, value]);

  // Honour readOnly flips at runtime.
  useEffect(() => {
    if (!editor) return;
    editor.setEditable(!readOnly);
  }, [editor, readOnly]);

  return (
    <>
      <EditorContent editor={editor} />
      <FormattingToolbar editor={editor} />
      <MobileToolbar editor={editor} />
      <BlockHandle editor={editor} editorRoot={editor?.view.dom.parentElement ?? null} />
      <SlashMenuPopover ref={slashPopoverRef} />
      <MentionPopover ref={mentionPopoverRef} />
      <NoteLinkPopover ref={noteLinkPopoverRef} onCreateNote={onCreateNote} />
    </>
  );
}

/** `tiptap-markdown` augments `editor.storage` with a markdown bag;
 * the package doesn't ship a TS declaration that flows through our
 * setup, so we narrow it here. */
function getMarkdown(editor: import("@tiptap/react").Editor): string {
  const storage = editor.storage as unknown as Record<string, unknown>;
  const md = storage.markdown as { getMarkdown?: () => string } | undefined;
  return md?.getMarkdown?.() ?? "";
}

