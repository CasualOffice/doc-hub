/**
 * NT5 Phase 1 — hover-revealed block handle.
 * Spec: docs/research/17-notes-general-user-ux.md §"Drag handle".
 *
 * Quiet hover affordance — a 6-dot handle fades in at the left margin
 * of whatever top-level block the cursor is over. Click opens a small
 * menu (Duplicate / Move up / Move down / Delete). Never visible by
 * default; never blocks the eye while reading.
 *
 * Desktop-only. Mobile gets the long-press block sheet instead
 * (NT6 Phase 2 — separate PIPELINE row).
 *
 * Phase 2 (separate PIPELINE row):
 *   - Drag-to-reorder via the handle (ProseMirror drag plumbing).
 *   - Turn into → sub-menu (Heading / List / Quote / Code).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { Editor } from "@tiptap/react";
import { GripVertical, Copy, ArrowUp, ArrowDown, Trash2 } from "lucide-react";
import { Popover } from "radix-ui";

interface Props {
  editor: Editor | null;
  /** Editor root — the element that wraps `<EditorContent>`. Mouse
   * tracking is scoped to this rect so the handle only appears when
   * the cursor is actually over the editor. */
  editorRoot: HTMLElement | null;
}

/** Resolved active block: the DOM rect we anchor the handle to, plus
 * the ProseMirror document position of the block's start. */
interface ActiveBlock {
  pos: number;
  rect: DOMRect;
}

export function BlockHandle({ editor, editorRoot }: Props) {
  const [active, setActive] = useState<ActiveBlock | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const rafRef = useRef<number | null>(null);

  // Hide handle whenever the menu closes by other means (esc, click-out).
  // The handle visibility tracks the cursor; if the cursor moves away
  // while the menu is open, we keep the handle visible until the menu
  // closes so the user doesn't see it flicker mid-interaction.
  const showHandle = active !== null;

  // Mouse tracking. Throttle to RAF so we don't recompute on every
  // pixel of movement.
  useEffect(() => {
    if (!editor || !editorRoot) return;

    const onMove = (e: MouseEvent) => {
      // While the menu is open we freeze the resolved block — the
      // user shouldn't see the handle skate away under their mouse.
      if (menuOpen) return;

      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      rafRef.current = requestAnimationFrame(() => {
        const root = editorRoot;
        if (!root) return;
        const rootRect = root.getBoundingClientRect();
        // Only resolve when the cursor is over the editor area
        // (within ~40 px to the left so the handle's hit area
        // counts).
        if (
          e.clientX < rootRect.left - 40 ||
          e.clientX > rootRect.right ||
          e.clientY < rootRect.top ||
          e.clientY > rootRect.bottom
        ) {
          setActive(null);
          return;
        }
        // Use posAtCoords with the rightward x so we hit the block
        // even when the cursor is in the empty left margin.
        const coordsX = Math.max(e.clientX, rootRect.left + 20);
        const found = editor.view.posAtCoords({ left: coordsX, top: e.clientY });
        if (!found) {
          setActive(null);
          return;
        }
        // Resolve to the top-level block ancestor.
        const doc = editor.view.state.doc;
        const $pos = doc.resolve(found.inside >= 0 ? found.inside : found.pos);
        // Walk up to depth 1 (immediate child of the doc).
        if ($pos.depth < 1) {
          setActive(null);
          return;
        }
        const blockPos = $pos.before(1);
        const blockNode = doc.nodeAt(blockPos);
        if (!blockNode) {
          setActive(null);
          return;
        }
        // Get the DOM node + rect for the resolved block.
        const dom = editor.view.nodeDOM(blockPos) as HTMLElement | null;
        if (!dom || dom.nodeType !== Node.ELEMENT_NODE) {
          setActive(null);
          return;
        }
        const rect = dom.getBoundingClientRect();
        setActive({ pos: blockPos, rect });
      });
    };

    const onLeave = () => {
      if (!menuOpen) setActive(null);
    };

    document.addEventListener("mousemove", onMove);
    editorRoot.addEventListener("mouseleave", onLeave);
    return () => {
      document.removeEventListener("mousemove", onMove);
      editorRoot.removeEventListener("mouseleave", onLeave);
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, [editor, editorRoot, menuOpen]);

  // ── Block operations ────────────────────────────────────────────
  const duplicateBlock = useCallback(() => {
    if (!editor || !active) return;
    const doc = editor.view.state.doc;
    const node = doc.nodeAt(active.pos);
    if (!node) return;
    const end = active.pos + node.nodeSize;
    editor
      .chain()
      .focus()
      .insertContentAt(end, node.toJSON(), { updateSelection: false })
      .run();
    setMenuOpen(false);
  }, [editor, active]);

  const moveBlock = useCallback(
    (direction: "up" | "down") => {
      if (!editor || !active) return;
      const doc = editor.view.state.doc;
      const node = doc.nodeAt(active.pos);
      if (!node) return;
      const end = active.pos + node.nodeSize;
      // Find sibling block boundary.
      if (direction === "up") {
        if (active.pos === 0) return;
        const $before = doc.resolve(active.pos);
        const prevPos = $before.before(1);
        if (prevPos < 0 || prevPos === active.pos) return;
        // Cut node, insert at prevPos.
        editor
          .chain()
          .focus()
          .insertContentAt(prevPos, node.toJSON(), { updateSelection: false })
          .deleteRange({ from: end + node.nodeSize, to: end + node.nodeSize * 2 })
          .run();
      } else {
        // direction === "down"
        const $end = doc.resolve(end);
        if ($end.nodeAfter == null) return;
        const nextNode = $end.nodeAfter;
        const nextEnd = end + nextNode.nodeSize;
        editor
          .chain()
          .focus()
          .insertContentAt(nextEnd, node.toJSON(), { updateSelection: false })
          .deleteRange({ from: active.pos, to: end })
          .run();
      }
      setMenuOpen(false);
    },
    [editor, active],
  );

  const deleteBlock = useCallback(() => {
    if (!editor || !active) return;
    const doc = editor.view.state.doc;
    const node = doc.nodeAt(active.pos);
    if (!node) return;
    editor
      .chain()
      .focus()
      .deleteRange({ from: active.pos, to: active.pos + node.nodeSize })
      .run();
    setMenuOpen(false);
    setActive(null);
  }, [editor, active]);

  if (!showHandle || !active) return null;

  // Position the handle ~22 px to the left of the block's left edge,
  // vertically centered with the block's first line height (~24 px).
  const handleLeft = active.rect.left - 28;
  const handleTop = active.rect.top + 4;

  return (
    <Popover.Root open={menuOpen} onOpenChange={setMenuOpen}>
      <Popover.Trigger asChild>
        <button
          type="button"
          aria-label="Block menu"
          className="cd-block-handle"
          style={{
            position: "fixed",
            left: handleLeft,
            top: handleTop,
            zIndex: 65,
          }}
          onMouseDown={(e) => {
            // Stop the editor from collapsing the selection when the
            // handle is clicked.
            e.preventDefault();
          }}
        >
          <GripVertical size={14} strokeWidth={2} />
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          side="bottom"
          align="start"
          sideOffset={4}
          className="cd-block-menu"
          onCloseAutoFocus={(e) => e.preventDefault()}
        >
          <BlockMenuItem icon={<Copy size={13} strokeWidth={1.8} />} label="Duplicate" onClick={duplicateBlock} />
          <BlockMenuItem icon={<ArrowUp size={13} strokeWidth={1.8} />} label="Move up" onClick={() => moveBlock("up")} />
          <BlockMenuItem icon={<ArrowDown size={13} strokeWidth={1.8} />} label="Move down" onClick={() => moveBlock("down")} />
          <div className="cd-block-menu-sep" role="separator" />
          <BlockMenuItem
            icon={<Trash2 size={13} strokeWidth={1.8} />}
            label="Delete"
            destructive
            onClick={deleteBlock}
          />
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

function BlockMenuItem({
  icon,
  label,
  onClick,
  destructive,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  destructive?: boolean;
}) {
  return (
    <button
      type="button"
      className={`cd-block-menu-item${destructive ? " is-destructive" : ""}`}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      <span className="cd-block-menu-icon">{icon}</span>
      <span>{label}</span>
    </button>
  );
}
