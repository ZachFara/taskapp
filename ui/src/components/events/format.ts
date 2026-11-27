import type { Event } from '../../lib/bindings';

/** The start day of an event as `YYYY-MM-DD` (times are local wall-clock text). */
export const startDay = (e: Event) => e.starts_at.slice(0, 10);
/** The last day it covers. */
export const endDay = (e: Event) => (e.ends_at ?? e.starts_at).slice(0, 10);

/** "09:30 to 10:30", "09:30" or "All day". */
export function timeSpan(e: Event): string {
  if (e.all_day) return 'All day';
  const from = e.starts_at.slice(11, 16);
  return e.ends_at && e.ends_at.slice(0, 10) === startDay(e) ? `${from} to ${e.ends_at.slice(11, 16)}` : from;
}

/** "Mon, Dec 10", formatted from the string so no zone shifts it. */
export const dayHeading = (d: string) =>
  new Date(`${d}T00:00:00Z`).toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric', timeZone: 'UTC' });

/** The calendar's own form of an event: all-day ends are exclusive, so they run to the day after. */
export function calendarSpan(e: Event): { start: string; end: string | undefined; allDay: boolean } {
  if (e.all_day) {
    const next = new Date(`${endDay(e)}T00:00:00Z`);
    next.setUTCDate(next.getUTCDate() + 1);
    return { start: startDay(e), end: next.toISOString().slice(0, 10), allDay: true };
  }
  return { start: e.starts_at, end: e.ends_at ?? undefined, allDay: false };
}
