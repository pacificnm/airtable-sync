import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { faCalendar, faChevronLeft, faChevronRight } from "../lib/fontawesome";
import { formatDisplayDate, todayIsoDate } from "../lib/date";
import { Icon } from "./Icon";

const WEEKDAY_LABELS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] as const;
const POPOVER_WIDTH_PX = 264;
const VIEWPORT_MARGIN_PX = 8;
const POPOVER_GAP_PX = 4;

type DatePickerProps = {
  value: string;
  onChange: (isoDate: string) => void;
  min?: string;
  max?: string;
  disabled?: boolean;
  className?: string;
  id?: string;
  placeholder?: string;
  variant?: "compact" | "default";
  placement?: "above" | "below";
};

type PopoverPosition = {
  top: number;
  left: number;
};

export function DatePicker({
  value,
  onChange,
  min,
  max,
  disabled = false,
  className = "",
  id,
  placeholder = "Select date",
  variant = "compact",
  placement = "above",
}: DatePickerProps) {
  const fallbackId = useId();
  const inputId = id ?? fallbackId;
  const anchorRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [popoverStyle, setPopoverStyle] = useState<PopoverPosition | null>(null);
  const [visibleMonth, setVisibleMonth] = useState(() => monthFromIso(value || todayIsoDate()));

  useEffect(() => {
    if (open) {
      setVisibleMonth(monthFromIso(value || todayIsoDate()));
    }
  }, [open, value]);

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) {
      setPopoverStyle(null);
      return;
    }

    const updatePosition = () => {
      const anchor = anchorRef.current;
      const popover = popoverRef.current;
      if (!anchor) {
        return;
      }

      const rect = anchor.getBoundingClientRect();
      const popoverHeight = popover?.offsetHeight ?? 280;
      const popoverWidth = popover?.offsetWidth ?? POPOVER_WIDTH_PX;
      const spaceAbove = rect.top - VIEWPORT_MARGIN_PX;
      const spaceBelow = window.innerHeight - rect.bottom - VIEWPORT_MARGIN_PX;

      let showAbove = placement === "above";
      if (showAbove && spaceAbove < popoverHeight && spaceBelow >= popoverHeight) {
        showAbove = false;
      } else if (!showAbove && spaceBelow < popoverHeight && spaceAbove >= popoverHeight) {
        showAbove = true;
      }

      let top = showAbove
        ? rect.top - popoverHeight - POPOVER_GAP_PX
        : rect.bottom + POPOVER_GAP_PX;
      let left = rect.left;

      top = Math.max(
        VIEWPORT_MARGIN_PX,
        Math.min(top, window.innerHeight - popoverHeight - VIEWPORT_MARGIN_PX),
      );
      left = Math.max(
        VIEWPORT_MARGIN_PX,
        Math.min(left, window.innerWidth - popoverWidth - VIEWPORT_MARGIN_PX),
      );

      setPopoverStyle({ top, left });
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open, placement, visibleMonth]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (anchorRef.current?.contains(target) || popoverRef.current?.contains(target)) {
        return;
      }
      setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const displayValue = value ? formatDisplayDate(value) : "";
  const isCompact = variant === "compact";
  const fieldClass = isCompact
    ? "px-2 py-1.5 text-xs rounded-l-nest-sm"
    : "px-3 py-2 text-sm rounded-l-nest-md";
  const iconButtonClass = isCompact
    ? "px-2 py-1.5 rounded-r-nest-sm"
    : "px-3 py-2 rounded-r-nest-md";

  const selectDate = (isoDate: string) => {
    onChange(isoDate);
    setOpen(false);
  };

  const popover =
    open && !disabled ? (
      <div
        ref={popoverRef}
        role="dialog"
        aria-label="Choose date"
        style={
          popoverStyle
            ? { top: popoverStyle.top, left: popoverStyle.left, width: POPOVER_WIDTH_PX }
            : { top: -9999, left: -9999, width: POPOVER_WIDTH_PX, visibility: "hidden" as const }
        }
        className="fixed z-[100] rounded-nest-md border border-nest-border bg-nest-surface p-3 shadow-lg"
      >
        <CalendarHeader
          year={visibleMonth.year}
          month={visibleMonth.month}
          onPrev={() =>
            setVisibleMonth((current) => shiftMonth(current.year, current.month, -1))
          }
          onNext={() =>
            setVisibleMonth((current) => shiftMonth(current.year, current.month, 1))
          }
        />
        <CalendarGrid
          year={visibleMonth.year}
          month={visibleMonth.month}
          value={value}
          min={min}
          max={max}
          onSelect={selectDate}
        />
        <div className="mt-2 flex justify-between border-t border-nest-border pt-2">
          <button
            type="button"
            onClick={() => selectDate(todayIsoDate())}
            className="text-[11px] font-medium text-nest-primary hover:underline"
          >
            Today
          </button>
          <button
            type="button"
            onClick={() => setOpen(false)}
            className="text-[11px] text-nest-muted hover:text-nest-foreground"
          >
            Close
          </button>
        </div>
      </div>
    ) : null;

  return (
    <div ref={anchorRef} className={`relative ${className}`}>
      <div className="flex">
        <button
          type="button"
          id={inputId}
          disabled={disabled}
          onClick={() => setOpen((current) => !current)}
          className={[
            "flex min-w-0 flex-1 items-center justify-between gap-2 border border-nest-border bg-nest-background text-left text-nest-foreground",
            fieldClass,
            disabled ? "cursor-not-allowed opacity-60" : "hover:border-nest-primary/40",
          ].join(" ")}
          aria-haspopup="dialog"
          aria-expanded={open}
        >
          <span className={displayValue ? "" : "text-nest-muted"}>
            {displayValue || placeholder}
          </span>
        </button>
        <button
          type="button"
          disabled={disabled}
          onClick={() => setOpen((current) => !current)}
          className={[
            "flex shrink-0 items-center justify-center border border-l-0 border-nest-border bg-nest-background text-nest-muted",
            iconButtonClass,
            disabled ? "cursor-not-allowed opacity-60" : "hover:bg-nest-primary/10 hover:text-nest-primary",
          ].join(" ")}
          aria-label="Open calendar"
        >
          <Icon icon={faCalendar} className="size-3.5" />
        </button>
      </div>

      {popover ? createPortal(popover, document.body) : null}
    </div>
  );
}

function CalendarHeader({
  year,
  month,
  onPrev,
  onNext,
}: {
  year: number;
  month: number;
  onPrev: () => void;
  onNext: () => void;
}) {
  const label = new Date(year, month - 1, 1, 12).toLocaleDateString("en-US", {
    month: "long",
    year: "numeric",
  });

  return (
    <div className="mb-2 flex items-center justify-between gap-2">
      <button
        type="button"
        onClick={onPrev}
        className="rounded-nest-sm p-1 text-nest-muted hover:bg-nest-background hover:text-nest-foreground"
        aria-label="Previous month"
      >
        <Icon icon={faChevronLeft} className="size-3" />
      </button>
      <span className="text-xs font-semibold text-nest-foreground">{label}</span>
      <button
        type="button"
        onClick={onNext}
        className="rounded-nest-sm p-1 text-nest-muted hover:bg-nest-background hover:text-nest-foreground"
        aria-label="Next month"
      >
        <Icon icon={faChevronRight} className="size-3" />
      </button>
    </div>
  );
}

function CalendarGrid({
  year,
  month,
  value,
  min,
  max,
  onSelect,
}: {
  year: number;
  month: number;
  value: string;
  min?: string;
  max?: string;
  onSelect: (isoDate: string) => void;
}) {
  const firstWeekday = new Date(year, month - 1, 1, 12).getDay();
  const totalDays = daysInMonth(year, month);
  const cells: Array<{ iso: string; day: number } | null> = [];

  for (let i = 0; i < firstWeekday; i += 1) {
    cells.push(null);
  }
  for (let day = 1; day <= totalDays; day += 1) {
    cells.push({ iso: isoFromParts(year, month, day), day });
  }

  return (
    <div>
      <div className="mb-1 grid grid-cols-7 gap-0.5">
        {WEEKDAY_LABELS.map((label) => (
          <div
            key={label}
            className="py-1 text-center text-[10px] font-medium uppercase text-nest-muted"
          >
            {label}
          </div>
        ))}
      </div>
      <div className="grid grid-cols-7 gap-0.5">
        {cells.map((cell, index) => {
          if (!cell) {
            return <div key={`empty-${index}`} aria-hidden />;
          }
          const disabled =
            (min !== undefined && compareIso(cell.iso, min) < 0) ||
            (max !== undefined && compareIso(cell.iso, max) > 0);
          const selected = value === cell.iso;
          const today = cell.iso === todayIsoDate();

          return (
            <button
              key={cell.iso}
              type="button"
              disabled={disabled}
              onClick={() => onSelect(cell.iso)}
              className={[
                "rounded-nest-sm py-1.5 text-[11px]",
                disabled
                  ? "cursor-not-allowed text-nest-muted/40"
                  : "text-nest-foreground hover:bg-nest-primary/10",
                selected ? "bg-nest-primary font-semibold text-white hover:bg-nest-primary" : "",
                today && !selected ? "ring-1 ring-inset ring-nest-primary/40" : "",
              ].join(" ")}
            >
              {cell.day}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function monthFromIso(iso: string) {
  const date = new Date(`${iso}T12:00:00`);
  if (Number.isNaN(date.getTime())) {
    const today = new Date();
    return { year: today.getFullYear(), month: today.getMonth() + 1 };
  }
  return { year: date.getFullYear(), month: date.getMonth() + 1 };
}

function shiftMonth(year: number, month: number, delta: number) {
  const next = new Date(year, month - 1 + delta, 1, 12);
  return { year: next.getFullYear(), month: next.getMonth() + 1 };
}

function isoFromParts(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function daysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}

function compareIso(a: string, b: string): number {
  return a.localeCompare(b);
}
