/**
 * Token-styled Radix prompt dialog. Single-field text input with
 * validation. Replaces `window.prompt` calls (browser-native, no
 * theme, ugly).
 */
import { useEffect, useRef, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";

interface Props {
  open: boolean;
  title: string;
  label?: string;
  placeholder?: string;
  /** Pre-filled value. Selected on open for one-keystroke replace. */
  defaultValue?: string;
  submitLabel?: string;
  cancelLabel?: string;
  /** Optional sync validator. Return a string error or null. */
  validate?: (v: string) => string | null;
  onSubmit: (value: string) => void | Promise<void>;
  onClose: () => void;
}

export function PromptDialog({
  open,
  title,
  label,
  placeholder,
  defaultValue = "",
  submitLabel = "Save",
  cancelLabel = "Cancel",
  validate,
  onSubmit,
  onClose,
}: Props) {
  const [value, setValue] = useState(defaultValue);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!open) return;
    setValue(defaultValue);
    setError(null);
    setSubmitting(false);
    requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.select();
    });
  }, [open, defaultValue]);

  async function submit() {
    if (submitting) return;
    const v = value.trim();
    const err = validate?.(v) ?? (v.length === 0 ? "Required" : null);
    if (err) {
      setError(err);
      return;
    }
    setSubmitting(true);
    try {
      await onSubmit(v);
      onClose();
    } catch {
      setSubmitting(false);
    }
  }

  return (
    <Dialog.Root open={open} onOpenChange={(o) => !o && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="cd-dialog-overlay" />
        <Dialog.Content className="cd-dialog-content">
          <div className="cd-dialog-header">
            <Dialog.Title className="cd-dialog-title">{title}</Dialog.Title>
          </div>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
          >
            {label && (
              <label
                htmlFor="cd-prompt-input"
                style={{
                  display: "block",
                  fontSize: "var(--text-xs)",
                  color: "var(--muted)",
                  marginBottom: 6,
                  letterSpacing: "0.04em",
                }}
              >
                {label}
              </label>
            )}
            <input
              ref={inputRef}
              id="cd-prompt-input"
              type="text"
              className="cd-dialog-input"
              placeholder={placeholder}
              value={value}
              aria-invalid={error !== null || undefined}
              onChange={(e) => {
                setValue(e.target.value);
                if (error) setError(null);
              }}
            />
            {error && (
              <div role="alert" style={{ marginTop: 6, fontSize: "var(--text-xs)", color: "var(--danger)" }}>
                {error}
              </div>
            )}
            <div className="cd-dialog-actions">
              <button
                type="button"
                className="cd-dialog-btn cd-dialog-btn--ghost"
                onClick={onClose}
              >
                {cancelLabel}
              </button>
              <button
                type="submit"
                className="cd-dialog-btn cd-dialog-btn--primary"
                disabled={submitting || value.trim().length === 0}
              >
                {submitting ? "Working…" : submitLabel}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
