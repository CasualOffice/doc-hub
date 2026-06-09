/**
 * NT4 — `+` note-link picker.
 * Spec: docs/research/17-notes-general-user-ux.md §"@ for people, [[ or + for notes".
 *
 * Typing `+` after whitespace (or at line start) opens a picker over
 * the active workspace's notes. Selecting one inserts the existing
 * markdown wiki-link token `[[Note title]]` — the backend's link
 * indexer already understands it and resolves the title to an id.
 *
 * Phase 2 (separate PIPELINE row): add `[[` as a parity trigger.
 * Tiptap-suggestion's `char` is single-character; multi-char triggers
 * need a custom prosemirror plugin. Until then, users who type
 * `[[Title]]` literally still get a wiki-link via the markdown parser.
 */
import { Extension } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import Suggestion, { type SuggestionOptions } from "@tiptap/suggestion";

import type { NoteNode } from "../../api/client.ts";

/** Distinct ProseMirror plugin key — see `slashMenu.ts` for the
 * rationale (default `suggestion$` collides across extensions). */
export const noteLinkPluginKey = new PluginKey("noteLinkSuggestion");

export interface NoteLinkItem {
  id: string;
  title: string;
}

export interface NoteLinkRendererControls {
  onUpdate: (state: {
    items: NoteLinkItem[];
    query: string;
    clientRect: (() => DOMRect | null) | null;
    command: (item: NoteLinkItem | "create") => void;
    /** When the query is non-empty and doesn't exactly match any
     * note, a "Create page «query»" footer row appears. The popover
     * receives this so it can render the row + dispatch `create`. */
    createDraft: string | null;
  }) => void;
  onExit: () => void;
  onKeyDown: (event: KeyboardEvent) => boolean;
}

interface ExtensionOpts {
  /** Resolver that returns the current workspace's notes tree. Called
   * every time the picker opens or filters, so it must be cheap (the
   * parent holds the tree in state already). */
  loadNotes: () => NoteNode[];
  controls: NoteLinkRendererControls;
}

export function filterNotes(tree: NoteNode[], query: string): NoteLinkItem[] {
  const q = query.trim().toLowerCase();
  const all = tree.map((n) => ({ id: n.id, title: n.title }));
  if (!q) return all.slice(0, 20);
  return all
    .filter((n) => n.title.toLowerCase().includes(q))
    .slice(0, 20);
}

export function noteLinkExtension(opts: ExtensionOpts): Extension {
  const suggestion: Omit<SuggestionOptions, "editor"> = {
    pluginKey: noteLinkPluginKey,
    char: "+",
    allowSpaces: false,
    startOfLine: false,
    items: ({ query }) => filterNotes(opts.loadNotes(), query),
    command: ({ editor, range, props }) => {
      const arg = props as NoteLinkItem | "create";
      if (arg === "create") {
        // Create-new is dispatched through the React popover with
        // its own draft text in scope; the extension only sees the
        // string sentinel and inserts a `[[Untitled]]` placeholder.
        // The popover composes the actual title into the editor
        // before dispatching, so this branch should be unreachable
        // in practice — defensively, do nothing.
        editor.chain().focus().deleteRange(range).run();
        return;
      }
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`[[${arg.title}]] `)
        .run();
    },
    render: () => {
      let lastQuery = "";
      return {
        onStart: (props) => {
          lastQuery = props.query;
          dispatchUpdate(opts.controls, props, lastQuery);
        },
        onUpdate: (props) => {
          lastQuery = props.query;
          dispatchUpdate(opts.controls, props, lastQuery);
        },
        onKeyDown: (props) => opts.controls.onKeyDown(props.event),
        onExit: () => opts.controls.onExit(),
      };
    },
  };

  return Extension.create({
    name: "noteLink",
    addProseMirrorPlugins() {
      return [Suggestion({ editor: this.editor, ...suggestion })];
    },
  });
}

function dispatchUpdate(
  controls: NoteLinkRendererControls,
  props: { items: unknown; query: string; clientRect?: (() => DOMRect | null) | null; command: (props: unknown) => void },
  query: string,
) {
  const items = props.items as NoteLinkItem[];
  const exact = items.some((it) => it.title.toLowerCase() === query.toLowerCase());
  controls.onUpdate({
    items,
    query,
    clientRect: props.clientRect ?? null,
    command: props.command as (item: NoteLinkItem | "create") => void,
    createDraft: query && !exact ? query : null,
  });
}
