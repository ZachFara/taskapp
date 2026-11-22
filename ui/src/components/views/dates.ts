import type { Issue, StatusCategory } from '../../lib/bindings';
import { localDay } from '../../lib/gantt';

/** The due day as the workspace zone sees it, so "which day" never depends on the UTC date. */
export function dueDay(i: Pick<Issue, 'due'>, tz: string): string | null {
  if (!i.due) return null;
  return i.due.Date ?? localDay(new Date(i.due.DateTime.utc), tz);
}

export function isOverdue(i: Pick<Issue, 'due'>, category: StatusCategory | undefined, tz: string, now = new Date()): boolean {
  const d = dueDay(i, tz);
  return d !== null && d < localDay(now, tz) && category !== 'Completed' && category !== 'Canceled';
}

/** Short "Dec 1" for a day string, formatted from the string itself so no zone shifts it. */
export const dayText = (d: string) =>
  new Date(`${d}T00:00:00Z`).toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
