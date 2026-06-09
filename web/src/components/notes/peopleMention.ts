/**
 * NT4 — `@` people-mention picker.
 * Spec: docs/research/17-notes-general-user-ux.md §"@ for people, [[ or + for notes".
 *
 * The extension owns the trigger detection + filter; a React popover
 * (see MentionPopover) renders the items via the controls bridge.
 *
 * Phase 1 inserts plain `@username ` text — the semantic mention node
 * lands alongside the notifications brief when it surfaces (NT4 Phase 2).
 */
import { Extension } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import Suggestion, { type SuggestionOptions } from "@tiptap/suggestion";

import type { WorkspaceMember } from "../../api/client.ts";

/** Distinct ProseMirror plugin key — see `slashMenu.ts` for the
 * rationale (default `suggestion$` collides across extensions). */
export const peopleMentionPluginKey = new PluginKey("peopleMentionSuggestion");

export interface MentionItem {
  id: string;
  username: string;
  is_admin: boolean;
}

export interface MentionRendererControls {
  onUpdate: (state: {
    items: MentionItem[];
    query: string;
    clientRect: (() => DOMRect | null) | null;
    command: (item: MentionItem) => void;
  }) => void;
  onExit: () => void;
  onKeyDown: (event: KeyboardEvent) => boolean;
}

interface ExtensionOpts {
  /** Resolver called every time the menu opens. Implementations cache
   * the result; the extension never knows the workspace id directly. */
  loadMembers: () => Promise<WorkspaceMember[]>;
  controls: MentionRendererControls;
}

/** Filter members by the partial query (case-insensitive prefix on
 * username; falls back to substring on full names if any). */
export function filterMembers(members: WorkspaceMember[], query: string): MentionItem[] {
  const q = query.trim().toLowerCase();
  const items = members.map((m) => ({
    id: m.user_id,
    username: m.username,
    is_admin: m.is_admin,
  }));
  if (!q) return items.slice(0, 20);
  return items
    .filter((m) => m.username.toLowerCase().includes(q))
    .slice(0, 20);
}

export function peopleMentionExtension(opts: ExtensionOpts): Extension {
  // Per-instance cache. Cleared by editor unmount.
  let cache: WorkspaceMember[] | null = null;
  const fetchOnce = async (): Promise<WorkspaceMember[]> => {
    if (cache) return cache;
    cache = await opts.loadMembers();
    return cache;
  };

  const suggestion: Omit<SuggestionOptions, "editor"> = {
    pluginKey: peopleMentionPluginKey,
    char: "@",
    allowSpaces: false,
    startOfLine: false,
    items: async ({ query }) => {
      const members = await fetchOnce().catch(() => []);
      return filterMembers(members, query);
    },
    command: ({ editor, range, props }) => {
      const m = props as MentionItem;
      // Insert plain `@username ` text. The trailing space exits
      // suggestion-mode cleanly. Backend doesn't care — this is a
      // human convention in the note body.
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`@${m.username} `)
        .run();
    },
    render: () => ({
      onStart: (props) => {
        opts.controls.onUpdate({
          items: props.items as MentionItem[],
          query: props.query,
          clientRect: props.clientRect ?? null,
          command: props.command as (item: MentionItem) => void,
        });
      },
      onUpdate: (props) => {
        opts.controls.onUpdate({
          items: props.items as MentionItem[],
          query: props.query,
          clientRect: props.clientRect ?? null,
          command: props.command as (item: MentionItem) => void,
        });
      },
      onKeyDown: (props) => opts.controls.onKeyDown(props.event),
      onExit: () => opts.controls.onExit(),
    }),
  };

  return Extension.create({
    name: "peopleMention",
    addProseMirrorPlugins() {
      return [Suggestion({ editor: this.editor, ...suggestion })];
    },
  });
}
