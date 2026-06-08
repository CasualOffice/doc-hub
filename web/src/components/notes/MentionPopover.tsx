/**
 * NT4 — `@` mention popover render. Pairs with peopleMentionExtension.
 * Keyboard navigation forwarded via the imperative handle.
 */
import { useCallback, useEffect, useImperativeHandle, useState, forwardRef } from "react";

import type { MentionItem } from "./peopleMention.ts";

export interface MentionPopoverHandle {
  onKeyDown: (e: KeyboardEvent) => boolean;
  update: (state: {
    items: MentionItem[];
    clientRect: (() => DOMRect | null) | null;
    command: (item: MentionItem) => void;
  }) => void;
  hide: () => void;
}

interface RenderedState {
  items: MentionItem[];
  clientRect: (() => DOMRect | null) | null;
  command: (item: MentionItem) => void;
}

export const MentionPopover = forwardRef<MentionPopoverHandle, {}>(
  function MentionPopover(_props, ref) {
    const [state, setState] = useState<RenderedState | null>(null);
    const [highlighted, setHighlighted] = useState(0);
    const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

    useEffect(() => {
      if (!state?.clientRect) {
        setPos(null);
        return;
      }
      const rect = state.clientRect();
      if (!rect) return;
      setPos({ left: rect.left, top: rect.bottom + 6 });
    }, [state]);

    useEffect(() => {
      setHighlighted(0);
    }, [state?.items]);

    const pick = useCallback(
      (index: number) => {
        if (!state) return;
        const item = state.items[index];
        if (!item) return;
        state.command(item);
      },
      [state],
    );

    useImperativeHandle(
      ref,
      () => ({
        update: (s) => setState(s),
        hide: () => setState(null),
        onKeyDown: (e: KeyboardEvent) => {
          if (!state || state.items.length === 0) return false;
          if (e.key === "ArrowDown") {
            setHighlighted((i) => (i + 1) % state.items.length);
            return true;
          }
          if (e.key === "ArrowUp") {
            setHighlighted((i) => (i - 1 + state.items.length) % state.items.length);
            return true;
          }
          if (e.key === "Enter") {
            pick(highlighted);
            return true;
          }
          if (e.key === "Escape") {
            setState(null);
            return true;
          }
          return false;
        },
      }),
      [state, highlighted, pick],
    );

    if (!state || !pos || state.items.length === 0) return null;

    return (
      <div
        className="cd-mention-menu"
        role="listbox"
        aria-label="Mention a member"
        style={{ position: "fixed", left: pos.left, top: pos.top, zIndex: 70 }}
      >
        {state.items.map((m, i) => (
          <button
            key={m.id}
            type="button"
            role="option"
            aria-selected={i === highlighted}
            className={`cd-mention-item${i === highlighted ? " is-active" : ""}`}
            onMouseEnter={() => setHighlighted(i)}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => pick(i)}
          >
            <span className="cd-mention-avatar">
              {m.username.charAt(0).toUpperCase()}
            </span>
            <span className="cd-mention-body">
              <span className="cd-mention-name">{m.username}</span>
              {m.is_admin && <span className="cd-mention-tag">Admin</span>}
            </span>
          </button>
        ))}
      </div>
    );
  },
);
