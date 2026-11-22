// Free-text dates and estimates for the command palette. Pure: "today" comes from the clock and zone passed in.
/** `null` means the text is not a value; `{ value: null }` means "clear it". */
export type Parsed<T> = { value: T | null } | null;

const DAY = 86_400_000;
const WEEKDAYS = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'];
const MONTHS = ['january', 'february', 'march', 'april', 'may', 'june', 'july', 'august', 'september', 'october', 'november', 'december'];

const iso = (ms: number) => new Date(ms).toISOString().slice(0, 10);
const ms = (d: string) => Date.parse(`${d}T00:00:00Z`);
const plus = (d: string, n: number) => iso(ms(d) + n * DAY);
const weekdayOf = (d: string) => (new Date(ms(d)).getUTCDay() + 6) % 7; // Monday is 0

/** The calendar day it is right now in `tz`. */
export const todayIn = (tz: string, now = new Date()) => now.toLocaleDateString('sv', { timeZone: tz });

const valid = (y: number, m: number, d: number) => {
  const t = new Date(Date.UTC(y, m - 1, d));
  return t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d ? iso(t.getTime()) : null;
};

/** The first day with this month and day on or after `today` (a year or four on, for 29 February). */
function monthDay(month: string, day: string, today: string) {
  const m = MONTHS.findIndex((x) => month.length >= 3 && x.startsWith(month)) + 1;
  if (!m || !/^\d{1,2}$/.test(day)) return null;
  const year = Number(today.slice(0, 4));
  for (let y = year; y <= year + 4; y++) {
    const d = valid(y, m, Number(day));
    if (d !== null && d >= today) return d;
  }
  return null;
}

/** `today`, `tomorrow`, `fri`, `next fri`, `nov 28`, `28 nov`, `2026-12-01`, `+3d`, `+2w`, `none`, as a day in `tz`. */
export function parseDate(text: string, tz: string, now = new Date()): Parsed<string> {
  const words = text.trim().toLowerCase().split(/\s+/);
  const today = todayIn(tz, now);
  const [a, b] = words;
  const weekday = (w = '') => WEEKDAYS.findIndex((x) => w.length >= 3 && x.startsWith(w));
  if (words.length === 1) {
    if (a === 'none') return { value: null };
    if (a === 'today') return { value: today };
    if (a === 'tomorrow') return { value: plus(today, 1) };
    if (weekday(a) >= 0) return { value: plus(today, ((weekday(a) - weekdayOf(today) + 6) % 7) + 1) };
    const ymd = /^(\d{4})-(\d{2})-(\d{2})$/.exec(a);
    if (ymd) {
      const d = valid(Number(ymd[1]), Number(ymd[2]), Number(ymd[3]));
      return d === null ? null : { value: d };
    }
    const rel = /^\+(\d+)([dw])$/.exec(a);
    return rel ? { value: plus(today, Number(rel[1]) * (rel[2] === 'w' ? 7 : 1)) } : null;
  }
  if (words.length !== 2) return null;
  if (a === 'next' && weekday(b) >= 0) return { value: plus(today, 7 - weekdayOf(today) + weekday(b)) };
  const d = /^\d/.test(a) ? monthDay(b, a, today) : monthDay(a, b, today);
  return d === null ? null : { value: d };
}

const NUM = String.raw`(\d+(?:\.\d+)?)`;
const SPAN = new RegExp(`^(?:${NUM}d)?(?:${NUM}h)?(?:${NUM}m)?$`);
/** `30m`, `2h`, `1h30m`, `1.5h`, `1d` (8 hours), `none`, as minutes. Units come in the order d, h, m, each once. */
export function parseEstimate(text: string): Parsed<number> {
  const t = text.trim().toLowerCase();
  if (t === 'none') return { value: null };
  const m = SPAN.exec(t);
  if (!m || t === '') return null;
  const minutes = (Number(m[1] ?? 0) * 8 + Number(m[2] ?? 0)) * 60 + Number(m[3] ?? 0);
  return Number.isInteger(minutes) && minutes > 0 && minutes <= 4_294_967_295 ? { value: minutes } : null;
}
