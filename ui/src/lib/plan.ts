import type { Issue, Status } from './bindings';
import { dueDay } from '../components/views/dates';

/** The planning column runs 07:00 to 22:00 on a 15-minute grid; times are minutes after midnight. */
export const FIRST = 7 * 60;
export const LAST = 22 * 60;
export const STEP = 15;
export const SLOTS = (LAST - FIRST) / STEP;

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));
const snap = (m: number) => Math.round(m / STEP) * STEP;

/** The grid minute under a pointer `y` px below the column top, given the column's pixel height. */
export const minuteAt = (y: number, height: number) => clamp(FIRST + snap((y / height) * (LAST - FIRST)), FIRST, LAST - STEP);

/** A start and length snapped to the grid and kept inside the column; the length is at least one step. */
export function fit(start: number, length: number): [number, number] {
  const len = clamp(snap(length), STEP, LAST - FIRST);
  const from = clamp(snap(start), FIRST, LAST - len);
  return [from, from + len];
}

export const clock = (m: number) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
export const stamp = (day: string, m: number) => `${day}T${clock(m)}:00`;
/** Minutes after midnight of an event's local time text. */
export const minutesOf = (iso: string) => Number(iso.slice(11, 13)) * 60 + Number(iso.slice(14, 16));
export const addDays = (day: string, n: number) => new Date(Date.parse(`${day}T00:00:00Z`) + n * 864e5).toISOString().slice(0, 10);

export type Candidates = { label: string; issues: Issue[] }[];

/** Open issues to plan from: Next actions (status Next or a project's flagged one), due within three days or overdue, then deep ones. Each issue appears once. */
export function candidates(issues: Issue[], statuses: Status[], nextIds: Set<number>, today: string, tz: string): Candidates {
  const open = issues.filter((i) => {
    const c = statuses.find((s) => s.id === i.status_id)?.category;
    return c !== 'Completed' && c !== 'Canceled';
  });
  const isNext = (i: Issue) => nextIds.has(i.id) || statuses.find((s) => s.id === i.status_id)?.name === 'Next';
  const soon = (i: Issue) => (dueDay(i, tz) ?? '9999') <= addDays(today, 3);
  const groups: Candidates = [
    { label: 'Next actions', issues: open.filter(isNext) },
    { label: 'Due soon', issues: open.filter((i) => !isNext(i) && soon(i)) },
    { label: 'Deep work', issues: open.filter((i) => !isNext(i) && !soon(i) && i.depth === 'deep') },
  ];
  return groups.filter((g) => g.issues.length);
}
