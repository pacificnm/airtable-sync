import { useEffect, useId } from "react";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { faTriangleExclamation, faXmark } from "../lib/fontawesome";
import { Icon } from "./Icon";

type ConfirmDialogProps = {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  icon?: IconDefinition;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  icon = faTriangleExclamation,
  danger = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const titleId = useId();
  const messageId = useId();

  useEffect(() => {
    if (!open) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onCancel();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onCancel]);

  if (!open) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4"
      onClick={onCancel}
      role="presentation"
    >
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={messageId}
        className="w-full max-w-md overflow-hidden rounded-nest-lg border border-nest-border bg-nest-surface shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="flex items-center justify-between border-b border-nest-border px-5 py-3">
          <div className="flex items-center gap-2">
            <Icon
              icon={icon}
              className={danger ? "size-4 text-nest-error" : "size-4 text-nest-warning"}
            />
            <h2 id={titleId} className="text-sm font-semibold">
              {title}
            </h2>
          </div>
          <button
            type="button"
            onClick={onCancel}
            className="rounded-nest-sm p-1 text-nest-muted hover:bg-nest-muted/10 hover:text-nest-foreground"
            aria-label="Close"
          >
            <Icon icon={faXmark} className="size-3.5" />
          </button>
        </header>

        <p id={messageId} className="whitespace-pre-line px-5 py-4 text-sm leading-relaxed text-nest-foreground">
          {message}
        </p>

        <footer className="flex justify-end gap-2 border-t border-nest-border px-5 py-3">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-nest-md border border-nest-border px-4 py-2 text-sm hover:bg-nest-muted/10"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            autoFocus
            onClick={onConfirm}
            className={[
              "rounded-nest-md px-4 py-2 text-sm font-medium text-white",
              danger
                ? "bg-nest-error hover:bg-nest-error/90"
                : "bg-nest-primary hover:bg-nest-secondary",
            ].join(" ")}
          >
            {confirmLabel}
          </button>
        </footer>
      </div>
    </div>
  );
}
