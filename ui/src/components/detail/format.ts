import type { Due, Issue } from '../../lib/bindings';

const pad = (n: number) => String(n).padStart(2, '0');

/** Local date and time of a due value, for the editors. A date-only due has no time. */
export function dueParts(due: Due | null): { date: string; time: string; timed: boolean } {
  if (due?.Date !== undefined) return { date: due.Date, time: '09:00', timed: false };
  if (due?.DateTime) {
    const d = new Date(due.DateTime.utc);
    return { date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`, time: `${pad(d.getHours())}:${pad(d.getMinutes())}`, timed: true };
  }
  return { date: '', time: '09:00', timed: false };
}

export const dueFrom = (date: string, time: string | null): Due | null =>
  !date ? null : time === null ? { Date: date } : { DateTime: { utc: new Date(`${date}T${time}`).toISOString(), tz: Intl.DateTimeFormat().resolvedOptions().timeZone } };

export const isDate = (s: string) => s === '' || /^\d{4}-\d{2}-\d{2}$/.test(s);
export const isTime = (s: string) => /^([01]\d|2[0-3]):[0-5]\d$/.test(s);

/** 90 -> "1h 30m". */
export function minutes(n: number): string {
  const h = Math.floor(n / 60);
  return h && n % 60 ? `${h}h ${n % 60}m` : h ? `${h}h` : `${n}m`;
}

export { PRIORITIES, priorityGlyph } from '../../lib/priority';

/** Share of sub-issues that are done, as whole percent. */
export const pct = (done: number, total: number) => (total ? Math.round((done / total) * 100) : 0);

export const dateOf = (iso: string) => iso.slice(0, 10);
export const sameDue = (a: Issue['due'], b: Issue['due']) => JSON.stringify(a) === JSON.stringify(b);

/** "Dec 10", or "Dec 10, 2027" when not this year; `empty` stands in for no date. `day` is a YYYY-MM-DD calendar day. */
export function humanDate(day: string | null, now = new Date(), empty = 'None'): string {
  if (!day) return empty;
  const [y, m, d] = day.split('-').map(Number);
  const year = y === now.getFullYear() ? {} : { year: 'numeric' as const };
  return new Intl.DateTimeFormat('en-US', { timeZone: 'UTC', month: 'short', day: 'numeric', ...year }).format(new Date(Date.UTC(y, m - 1, d)));
}

/** "Mon, Nov 30": the weekday and day of a YYYY-MM-DD day, read from the string so no zone shifts it. */
export const comesBackText = (day: string) =>
  new Date(`${day}T00:00:00Z`).toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric', timeZone: 'UTC' });
