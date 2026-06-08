/**
 * Token-styled Radix confirm dialog. Replaces `window.confirm` calls
 * throughout the SPA — `window.confirm` ships a browser-native modal
 * (system font, OS-shaped buttons, no theme, no a11y polish) which
 * isn't the premium UX bar the rest of Drive holds.
 *
 * Usage:
 *   <ConfirmDialog
 *     open={open}
 *     title="Move 3 files to trash?"
 *     body="They'll stay in Trash for 30 days before being removed."
 *     confirmLabel="Move to trash"
 *     variant="destructive"
 *     onConfirm={async () => await trashThem()}
 *     onClose={() => setOpen(false)}
 *   />
 */
import { useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { AlertTriangle } from "lucide-react";

interface Props {
  open: boolean;
  title: string;
  body?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** `destructive` → red confirm button + warning icon header. */
  variant?: "default" | "destructive";
  onConfirm: () => void | Promise<void>;
  onClose: () => void;
}

export function ConfirmDialog({
  open,
  title,
  body,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  variant = "default",
  onConfirm,
  onClose,
}: Props) {
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (open) setSubmitting(false);
  }, [open]);

  async function submit() {
    if (submitting) return;
    setSubmitting(true);
    try {
      await onConfirm();
      onClose();
    } catch {
      // Caller is responsible for surfacing the error (toast etc).
      // We just re-enable the button so the user can retry.
      setSubmitting(false);
    }
  }

  const isDestructive = variant === "destructive";

  return (
    <Dialog.Root open={open} onOpenChange={(o) => !o && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="cd-dialog-overlay" />
        <Dialog.Content
          className="cd-dialog-content"
          onOpenAutoFocus={(e) => {
            // Focus the confirm button on open so Enter submits.
            e.preventDefault();
            const btn = document.getElementById("cd-confirm-btn");
            btn?.focus();
          }}
        >
          <div className="cd-dialog-header">
            {isDestructive && (
              <span className="cd-dialog-icon cd-dialog-icon--warn" aria-hidden="true">
                <AlertTriangle size={16} strokeWidth={2} />
              </span>
            )}
            <Dialog.Title className="cd-dialog-title">{title}</Dialog.Title>
          </div>
          {body && <Dialog.Description className="cd-dialog-body">{body}</Dialog.Description>}
          <div className="cd-dialog-actions">
            <button type="button" className="cd-dialog-btn cd-dialog-btn--ghost" onClick={onClose}>
              {cancelLabel}
            </button>
            <button
              type="button"
              id="cd-confirm-btn"
              className={`cd-dialog-btn ${isDestructive ? "cd-dialog-btn--danger" : "cd-dialog-btn--primary"}`}
              disabled={submitting}
              onClick={() => void submit()}
            >
              {submitting ? "Working…" : confirmLabel}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
