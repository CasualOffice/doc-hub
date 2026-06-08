/**
 * NT4 — `+` note-link popover. Renders matching notes + an optional
 * "Create page «query»" footer when the query doesn't exactly match.
 */
import { useCallback, useEffect, useImperativeHandle, useState, forwardRef } from "react";
import { FileText, Plus } from "lucide-react";

import type { NoteLinkItem } from "./noteLink.ts";

export interface NoteLinkPopoverHandle {
  onKeyDown: (e: KeyboardEvent) => boolean;
  update: (state: {
    items: NoteLinkItem[];
    clientRect: (() => DOMRect | null) | null;
    command: (item: NoteLinkItem | "create") => void;
    createDraft: string | null;
  }) => void;
  hide: () => void;
}

interface RenderedState {
  items: NoteLinkItem[];
  clientRect: (() => DOMRect | null) | null;
  command: (item: NoteLinkItem | "create") => void;
  createDraft: string | null;
}

interface Props {
  /** Optional handler when the user picks "Create page «query»".
   * Receives the typed title; the parent creates the note + the
   * extension inserts the wiki-link with that title. */
  onCreateNote?: (title: string) => void;
}

export const NoteLinkPopover = forwardRef<NoteLinkPopoverHandle, Props>(
  function NoteLinkPopover({ onCreateNote }, ref) {
    const [state, setState] = useState<RenderedState | null>(null);
    const [highlighted, setHighlighted] = useState(0);
    const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

    const visibleCount =
      (state?.items.length ?? 0) + (state?.createDraft ? 1 : 0);

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
    }, [state?.items, state?.createDraft]);

    const pick = useCallback(
      (index: number) => {
        if (!state) return;
        if (index < state.items.length) {
          state.command(state.items[index]);
        } else if (state.createDraft) {
          // Delegate to the parent — it creates the note then we
          // let the extension insert `[[Title]]`.
          onCreateNote?.(state.createDraft);
          // Also hide locally so we don't re-fire on subsequent
          // updates from the extension.
          setState(null);
        }
      },
      [state, onCreateNote],
    );

    useImperativeHandle(
      ref,
      () => ({
        update: (s) => setState(s),
        hide: () => setState(null),
        onKeyDown: (e: KeyboardEvent) => {
          if (!state || visibleCount === 0) return false;
          if (e.key === "ArrowDown") {
            setHighlighted((i) => (i + 1) % visibleCount);
            return true;
          }
          if (e.key === "ArrowUp") {
            setHighlighted((i) => (i - 1 + visibleCount) % visibleCount);
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
      [state, highlighted, pick, visibleCount],
    );

    if (!state || !pos || visibleCount === 0) return null;

    return (
      <div
        className="cd-mention-menu"
        role="listbox"
        aria-label="Link to a note"
        style={{ position: "fixed", left: pos.left, top: pos.top, zIndex: 70 }}
      >
        {state.items.map((n, i) => (
          <button
            key={n.id}
            type="button"
            role="option"
            aria-selected={i === highlighted}
            className={`cd-mention-item${i === highlighted ? " is-active" : ""}`}
            onMouseEnter={() => setHighlighted(i)}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => pick(i)}
          >
            <span className="cd-mention-avatar cd-mention-avatar--note">
              <FileText size={12} strokeWidth={1.8} />
            </span>
            <span className="cd-mention-body">
              <span className="cd-mention-name">{n.title}</span>
            </span>
          </button>
        ))}
        {state.createDraft && (
          <button
            type="button"
            role="option"
            aria-selected={highlighted === state.items.length}
            className={`cd-mention-item cd-mention-create${
              highlighted === state.items.length ? " is-active" : ""
            }`}
            onMouseEnter={() => setHighlighted(state.items.length)}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => pick(state.items.length)}
          >
            <span className="cd-mention-avatar cd-mention-avatar--create">
              <Plus size={12} strokeWidth={2} />
            </span>
            <span className="cd-mention-body">
              <span className="cd-mention-name">Create page "{state.createDraft}"</span>
            </span>
          </button>
        )}
      </div>
    );
  },
);
