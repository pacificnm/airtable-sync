import type { ReactNode } from "react";
import { useStatusBar, type StatusVariant } from "../context/StatusBarContext";

const VARIANT_CLASS: Record<StatusVariant, string> = {
  info: "text-nest-info",
  success: "text-nest-success",
  warning: "text-nest-warning",
  error: "text-nest-error",
};

type StatusBarProps = {
  /** Optional fixed-width content pinned to the left (e.g. counts). */
  left?: ReactNode;
  /** Optional fixed-width content pinned to the right (e.g. view + zoom). */
  right?: ReactNode;
};

/**
 * Footer status bar. The center region shows the live message from
 * {@link useStatusBar}; `left`/`right` slots are app-specific.
 */
export function StatusBar({ left, right }: StatusBarProps) {
  const { message, variant } = useStatusBar();

  return (
    <footer
      className="flex h-6 shrink-0 items-stretch border-t border-nest-border bg-nest-surface text-[11px]"
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      {left ? (
        <div className="flex shrink-0 items-center border-r border-nest-border px-2 font-medium text-nest-foreground">
          {left}
        </div>
      ) : null}
      <p
        className={`flex min-w-0 flex-1 items-center truncate px-2 ${VARIANT_CLASS[variant]}`}
      >
        {message}
      </p>
      {right ? (
        <div className="flex shrink-0 items-center justify-end gap-2 border-l border-nest-border px-2 font-medium text-nest-foreground">
          {right}
        </div>
      ) : null}
    </footer>
  );
}
