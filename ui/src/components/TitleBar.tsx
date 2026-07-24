import { WindowControls } from "./WindowControls";
import { isTauri } from "../lib/tauri";

type TitleBarProps = {
  /** Centered window title (usually the app's display name). */
  title: string;
};

/**
 * Frameless title bar: draggable strip, centered app title, window controls.
 * Pairs with `"decorations": false` in `tauri.conf.json`.
 */
export function TitleBar({ title }: TitleBarProps) {
  const showWindowChrome = isTauri();

  return (
    <header className="relative flex h-8 shrink-0 items-stretch border-b border-nest-border bg-nest-surface text-[13px]">
      {showWindowChrome ? (
        <div className="min-w-0 flex-1" data-tauri-drag-region />
      ) : (
        <div className="min-w-0 flex-1" />
      )}

      <div className="relative z-10 flex h-full shrink-0 items-stretch">
        {showWindowChrome ? <WindowControls /> : null}
      </div>

      <p
        className="pointer-events-none absolute inset-0 z-0 flex items-center justify-center px-28"
        aria-hidden
      >
        <span className="truncate text-[12px] font-medium text-nest-foreground">{title}</span>
      </p>
    </header>
  );
}
