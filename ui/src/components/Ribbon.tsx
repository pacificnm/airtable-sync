import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import type { ReactNode } from "react";
import { Icon } from "./Icon";

/** A ribbon tab identifier. Apps typically use a string-literal union. */
export type RibbonTabId = string;

export type RibbonTabDef = {
  id: RibbonTabId;
  label: string;
};

type RibbonProps = {
  /** Tabs to render, left to right. */
  tabs: RibbonTabDef[];
  activeTab: RibbonTabId;
  onTabChange: (tab: RibbonTabId) => void;
  /** Tab id styled like the MS-Office "File" tab (secondary accent). */
  fileTabId?: RibbonTabId;
  /** Active tab's group/button content. */
  children: ReactNode;
};

export function Ribbon({
  tabs,
  activeTab,
  onTabChange,
  fileTabId = "file",
  children,
}: RibbonProps) {
  return (
    <div className="shrink-0 bg-nest-surface">
      <div
        className="flex items-end gap-0 border-b border-nest-border bg-nest-surface px-1 pt-0.5"
        role="tablist"
        aria-label="Ribbon"
      >
        {tabs.map((tab) => {
          const selected = activeTab === tab.id;
          const isFile = tab.id === fileTabId;
          return (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={selected}
              className={[
                "relative mb-[-1px] rounded-t-nest-sm px-3 py-1 text-xs transition-colors",
                selected
                  ? isFile
                    ? "z-10 border border-b-0 border-nest-border bg-nest-background font-semibold text-nest-secondary"
                    : "z-10 border border-b-0 border-nest-border bg-nest-background font-semibold text-nest-primary"
                  : "text-nest-foreground hover:bg-nest-muted/10",
              ].join(" ")}
              onClick={() => onTabChange(tab.id)}
            >
              {tab.label}
            </button>
          );
        })}
      </div>
      <div
        className="h-[4.5rem] overflow-hidden border-b border-nest-border bg-nest-background px-2 py-1"
        role="tabpanel"
      >
        {children}
      </div>
    </div>
  );
}

type RibbonGroupProps = {
  label: string;
  hideLabel?: boolean;
  children: ReactNode;
};

export function RibbonGroup({ label, children, hideLabel = false }: RibbonGroupProps) {
  return (
    <div className="flex h-[3.75rem] flex-col border-r border-nest-border px-1.5 last:border-r-0">
      <div className="flex h-[3.25rem] items-center gap-0.5">{children}</div>
      <div className="flex h-3 shrink-0 items-center border-t border-nest-border/80 pt-px">
        {hideLabel ? (
          <span className="sr-only">{label}</span>
        ) : (
          <span className="block w-full truncate text-center text-[10px] leading-none text-nest-muted">
            {label}
          </span>
        )}
      </div>
    </div>
  );
}

export type RibbonIconTint =
  | "primary"
  | "secondary"
  | "accent"
  | "info"
  | "warning"
  | "neutral";

const ICON_TINT_CLASS: Record<RibbonIconTint, string> = {
  primary: "bg-nest-primary/12 text-nest-primary",
  secondary: "bg-nest-secondary/12 text-nest-secondary",
  accent: "bg-nest-accent/20 text-[#3d6b12]",
  info: "bg-sky-100 text-sky-700",
  warning: "bg-amber-100 text-amber-800",
  neutral: "bg-nest-muted/15 text-nest-muted",
};

type RibbonButtonProps = {
  label: string;
  icon?: IconDefinition;
  onClick?: () => void;
  disabled?: boolean;
  large?: boolean;
  active?: boolean;
  iconTint?: RibbonIconTint;
};

export function RibbonButton({
  label,
  icon,
  onClick,
  disabled = false,
  large = false,
  active = false,
  iconTint = "primary",
}: RibbonButtonProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      title={label}
      className={[
        "flex flex-col items-center justify-center rounded-nest-sm border text-center transition-colors",
        large ? "h-[3.25rem] min-w-[4rem] px-1.5 py-0.5" : "h-[3.25rem] min-w-[3rem] px-1 py-0.5",
        disabled
          ? "cursor-not-allowed border-transparent opacity-50"
          : active
            ? "border-nest-primary/35 bg-nest-primary/10 text-nest-primary"
            : "border-transparent text-nest-foreground hover:border-nest-border hover:bg-nest-surface hover:shadow-sm",
      ].join(" ")}
    >
      {icon ? (
        <span
          className={[
            "mb-1 flex shrink-0 items-center justify-center rounded-nest-sm",
            large ? "size-7" : "size-5",
            disabled ? "bg-nest-muted/10 text-nest-muted" : ICON_TINT_CLASS[iconTint],
          ].join(" ")}
        >
          <Icon icon={icon} className={large ? "size-4" : "size-3"} />
        </span>
      ) : null}
      <span
        className={[
          "max-w-[5.5rem] leading-tight",
          large ? "text-[10px]" : "text-[9px]",
        ].join(" ")}
      >
        {label}
      </span>
    </button>
  );
}

/** Vertically stacked ribbon commands (MS Project style). */
export function RibbonButtonStack({ children }: { children: ReactNode }) {
  return <div className="flex h-[3.25rem] flex-col justify-center gap-0.5">{children}</div>;
}

type RibbonMenuButtonProps = {
  label: string;
  icon?: IconDefinition;
  onClick?: () => void;
  disabled?: boolean;
  active?: boolean;
  iconTint?: RibbonIconTint;
};

export function RibbonMenuButton({
  label,
  icon,
  onClick,
  disabled = false,
  active = false,
  iconTint = "neutral",
}: RibbonMenuButtonProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      title={label}
      className={[
        "flex h-[1.125rem] min-w-[6.5rem] items-center gap-1 rounded-nest-sm border px-1 text-left text-[9px] transition-colors",
        disabled
          ? "cursor-not-allowed opacity-50"
          : active
            ? "border-nest-primary/35 bg-nest-primary/10 text-nest-primary"
            : "border-transparent hover:border-nest-border hover:bg-nest-surface",
      ].join(" ")}
    >
      {icon ? (
        <span
          className={[
            "flex size-4 shrink-0 items-center justify-center rounded-nest-sm",
            disabled ? "bg-nest-muted/10 text-nest-muted" : ICON_TINT_CLASS[iconTint],
          ].join(" ")}
        >
          <Icon icon={icon} className="size-2.5" />
        </span>
      ) : null}
      <span className="truncate">{label}</span>
    </button>
  );
}
