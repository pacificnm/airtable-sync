/** Date helpers shared by the desktop shell (e.g. `DatePicker`). */

/** Formats an ISO `yyyy-mm-dd` date for display (e.g. "Jan 5, 2026"). */
export function formatDisplayDate(isoDate: string): string {
  const date = new Date(`${isoDate}T12:00:00`);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

/** Returns today's date as ISO `yyyy-mm-dd`. */
export function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10);
}
