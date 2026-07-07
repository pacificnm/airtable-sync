import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faCircleCheck,
  faCircleExclamation,
  faCircleInfo,
  faTriangleExclamation,
  faXmark,
} from "../lib/fontawesome";
import { Icon } from "./Icon";
import { useToast, type ToastItem, type ToastVariant } from "../context/ToastContext";

const VARIANT_STYLES: Record<
  ToastVariant,
  { container: string; icon: IconDefinition }
> = {
  success: {
    container: "border-nest-success/30 bg-nest-success text-white",
    icon: faCircleCheck,
  },
  info: {
    container: "border-nest-info/30 bg-nest-info text-white",
    icon: faCircleInfo,
  },
  warning: {
    container: "border-nest-warning/30 bg-nest-warning text-white",
    icon: faTriangleExclamation,
  },
  error: {
    container: "border-nest-error/30 bg-nest-error text-white",
    icon: faCircleExclamation,
  },
};

export function ToastViewport() {
  const { toasts } = useToast();

  if (toasts.length === 0) {
    return null;
  }

  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
      aria-live="polite"
      aria-relevant="additions"
    >
      {toasts.map((toast) => (
        <ToastCard key={toast.id} toast={toast} />
      ))}
    </div>
  );
}

function ToastCard({ toast }: { toast: ToastItem }) {
  const { dismiss } = useToast();
  const style = VARIANT_STYLES[toast.variant];

  return (
    <div
      role="status"
      className={[
        "toast-enter pointer-events-auto flex items-start gap-2 rounded-nest-md border px-3 py-2.5 shadow-lg",
        style.container,
      ].join(" ")}
    >
      <Icon icon={style.icon} className="mt-0.5 size-4 shrink-0 opacity-95" />
      <p className="min-w-0 flex-1 text-sm leading-snug">{toast.message}</p>
      <button
        type="button"
        onClick={() => dismiss(toast.id)}
        className="shrink-0 rounded-nest-sm p-0.5 opacity-80 hover:bg-white/15 hover:opacity-100"
        aria-label="Dismiss notification"
      >
        <Icon icon={faXmark} className="size-3.5" />
      </button>
    </div>
  );
}
