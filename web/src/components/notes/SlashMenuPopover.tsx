/**
 * NT3 — slash menu popover render.
 * Renders the items the Tiptap suggestion plugin surfaces via
 * `slashMenuExtension`'s controls. Keyboard navigation is handled here
 * (Arrow up/down + Enter + Escape) and forwarded back into the editor
 * via the `onKeyDown` hook.
 */
import { useCallback, useEffect, useImperativeHandle, useState, forwardRef } from "react";
import { Heading1, Heading2, Heading3, List, ListOrdered, Quote, Code, Minus } from "lucide-react";

import type { SlashItem } from "./slashMenu.ts";

export interface SlashPopoverHandle {
  /** Forward the editor keydown into the popover. Return true if the
   * popover handled it (so Tiptap stops propagation). */
  onKeyDown: (e: KeyboardEvent) => boolean;
  /** Replace the visible item list (suggestion plugin re-filtered). */
  update: (state: {
    items: SlashItem[];
    clientRect: (() => DOMRect | null) | null;
    command: (item: SlashItem) => void;
  }) => void;
  /** Hide. */
  hide: () => void;
}

interface RenderedState {
  items: SlashItem[];
  clientRect: (() => DOMRect | null) | null;
  command: (item: SlashItem) => void;
}

export const SlashMenuPopover = forwardRef<SlashPopoverHandle, {}>(
  function SlashMenuPopover(_props, ref) {
    const [state, setState] = useState<RenderedState | null>(null);
    const [highlighted, setHighlighted] = useState(0);
    const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

    // Reposition on every update so the popover tracks the caret.
    useEffect(() => {
      if (!state?.clientRect) {
        setPos(null);
        return;
      }
      const rect = state.clientRect();
      if (!rect) {
        setPos(null);
        return;
      }
      setPos({
        // Place just below the caret; the user can scroll inside the
        // popover with the keyboard if the list is long.
        left: rect.left,
        top: rect.bottom + 6,
      });
    }, [state]);

    // Reset highlight when items change.
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
        className="cd-slash-menu"
        role="listbox"
        aria-label="Insert block"
        style={{ position: "fixed", left: pos.left, top: pos.top, zIndex: 70 }}
      >
        {state.items.map((item, i) => (
          <button
            key={item.id}
            type="button"
            role="option"
            aria-selected={i === highlighted}
            className={`cd-slash-item${i === highlighted ? " is-active" : ""}`}
            onMouseEnter={() => setHighlighted(i)}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => pick(i)}
          >
            <span className="cd-slash-icon">{iconFor(item.id)}</span>
            <span className="cd-slash-body">
              <span className="cd-slash-title">{item.title}</span>
              {item.description && (
                <span className="cd-slash-desc">{item.description}</span>
              )}
            </span>
          </button>
        ))}
      </div>
    );
  },
);

function iconFor(id: string): React.ReactNode {
  switch (id) {
    case "h1":
      return <Heading1 size={14} strokeWidth={1.8} />;
    case "h2":
      return <Heading2 size={14} strokeWidth={1.8} />;
    case "h3":
      return <Heading3 size={14} strokeWidth={1.8} />;
    case "ul":
      return <List size={14} strokeWidth={1.8} />;
    case "ol":
      return <ListOrdered size={14} strokeWidth={1.8} />;
    case "quote":
      return <Quote size={14} strokeWidth={1.8} />;
    case "code":
      return <Code size={14} strokeWidth={1.8} />;
    case "hr":
      return <Minus size={14} strokeWidth={1.8} />;
    default:
      return null;
  }
}
