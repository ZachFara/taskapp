// Port of crates/core/src/quickadd.rs `parse`, so the mock reads quick-add text exactly as the backend does.
import type { Due, ParsedQuickAdd } from './bindings';

const DAY = 86_400_000;
const iso = (d: Date) => d.toISOString().slice(0, 10);
const addDays = (d: string, n: number) => iso(new Date(Date.parse(`${d}T00:00:00Z`) + n * DAY));
const weekdayOf = (d: string) => (new Date(`${d}T00:00:00Z`).getUTCDay() + 6) % 7; // Monday = 0
const WEEKDAYS: Record<string, number> = {
  mon: 0, monday: 0, tue: 1, tuesday: 1, wed: 2, wednesday: 2, thu: 3, thursday: 3,
  fri: 4, friday: 4, sat: 5, saturday: 5, sun: 6, sunday: 6,
};
const MONTHS = ['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec'];

/** Next occurrence after `today`; today's own weekday means seven days out. */
function nextWeekday(today: string, wd: number) {
  const diff = (wd + 7 - weekdayOf(today)) % 7;
  return addDays(today, diff === 0 ? 7 : diff);
}

const validDate = (y: number, m: number, d: number) => {
  const t = new Date(Date.UTC(y, m - 1, d));
  return t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d ? iso(t) : null;
};

function monthDay(tokens: string[], i: number, today: string): [string, number] | null {
  const first = tokens[i]?.toLowerCase();
  if (first === undefined) return null;
  const isNum = /^\d+$/.test(first);
  const [name, day] = isNum ? [tokens[i + 1]?.toLowerCase(), Number(first)] : [first, /^\d+$/.test(tokens[i + 1] ?? '') ? Number(tokens[i + 1]) : NaN];
  const m = MONTHS.indexOf(name ?? '') + 1;
  if (m === 0 || Number.isNaN(day)) return null;
  const year = Number(today.slice(0, 4));
  const thisYear = validDate(year, m, day);
  if (thisYear === null) return null;
  return [thisYear < today ? validDate(year + 1, m, day) ?? thisYear : thisYear, 2];
}

function dateAt(tokens: string[], i: number, today: string): [string, number] | null {
  const w = tokens[i]?.toLowerCase();
  if (w === undefined) return null;
  if (w === 'today') return [today, 1];
  if (w === 'tomorrow') return [addDays(today, 1), 1];
  if (w === 'next' && tokens[i + 1]?.toLowerCase() === 'week') return [nextWeekday(today, 0), 2];
  if (w === 'in') {
    if (!/^\d+$/.test(tokens[i + 1] ?? '')) return null;
    const unit = tokens[i + 2]?.toLowerCase();
    const per = unit === 'day' || unit === 'days' ? 1 : unit === 'week' || unit === 'weeks' ? 7 : 0;
    return per ? [addDays(today, Number(tokens[i + 1]) * per), 3] : null;
  }
  if (w in WEEKDAYS) return [nextWeekday(today, WEEKDAYS[w]), 1];
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(w);
  const parsed = m && validDate(Number(m[1]), Number(m[2]), Number(m[3]));
  return parsed ? [parsed, 1] : monthDay(tokens, i, today);
}

/** `3pm`, `9:15am`, `15:30`. A bare hour needs am/pm or a colon. Returns [hour, minute]. */
function timeAt(token: string | undefined): [number, number] | null {
  const s = token?.toLowerCase();
  if (s === undefined) return null;
  const mer = s.endsWith('am') ? 'am' : s.endsWith('pm') ? 'pm' : null;
  const body = mer ? s.slice(0, -2) : s;
  const parts = body.split(':');
  if (parts.length > 2 || parts.some((p) => !/^\d+$/.test(p))) return null;
  const [h, m] = [Number(parts[0]), parts.length === 2 ? Number(parts[1]) : 0];
  if (parts.length === 1 && !mer) return null;
  if (m > 59) return null;
  if (mer) return h >= 1 && h <= 12 ? [(h % 12) + (mer === 'pm' ? 12 : 0), m] : null;
  return h <= 23 ? [h, m] : null;
}

const offsetAt = (ms: number, tz: string) => {
  const p = Object.fromEntries(
    new Intl.DateTimeFormat('en-US', { timeZone: tz, hourCycle: 'h23', year: 'numeric', month: 'numeric', day: 'numeric', hour: 'numeric', minute: 'numeric', second: 'numeric' })
      .formatToParts(new Date(ms)).map((x) => [x.type, Number(x.value)]),
  );
  return Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second) - Math.floor(ms / 1000) * 1000;
};

/** The earliest instant whose wall clock in `tz` reads `date` `time`, or null when that time does not exist there. */
function toDue(date: string, [h, m]: [number, number], tz: string): Due | null {
  const wall = Date.parse(`${date}T00:00:00Z`) + (h * 60 + m) * 60_000;
  const guesses = [wall - offsetAt(wall, tz), wall - offsetAt(wall - DAY, tz), wall - offsetAt(wall + DAY, tz)];
  const hits = guesses.filter((g) => g + offsetAt(g, tz) === wall).sort((a, b) => a - b);
  return hits.length ? { DateTime: { utc: new Date(hits[0]).toISOString().replace('.000Z', 'Z'), tz } } : null;
}

const priority = (w: string) => (/^p[0-4]$/.test(w) ? Number(w[1]) : null);
const estimate = (w: string) => {
  const m = /^~(\d+)([mhd])$/.exec(w);
  const n = m ? Number(m[1]) * { m: 1, h: 60, d: 480 }[m[2] as 'm' | 'h' | 'd'] : 0;
  return n > 0 && n <= 4_294_967_295 ? n : null;
};

/** `3`, `0.5` or `.5`, within what a stored value may be (core: `points_number`). */
const pointsNumber = (n: string) => {
  const m = /^(\d*)(?:\.(\d))?$/.exec(n);
  const p = m && (m[1] !== '' || m[2] !== undefined) ? Number(n) : NaN;
  return p >= 0 && p <= 1000 && Number(p.toFixed(1)) === p ? p : null;
};
/** `3pt`, `3pts`, `.5pt` or `pt3`. */
const points = (w: string) => {
  const n = /^(.*)pts?$/.exec(w)?.[1] ?? (w.startsWith('pt') ? w.slice(2) : null);
  return n === null ? null : pointsNumber(n);
};

export function parseQuickadd(input: string, today: string, tz: string, pointsOn = true): ParsedQuickAdd {
  const tokens = input.split(/\s+/).filter(Boolean);
  const out = { dueDate: null as string | null, time: null as [[number, number], string] | null, start: null as string | null, priority: null as number | null, slug: null as string | null, labels: [] as string[], estimate: null as number | null, points: null as number | null };
  const title: string[] = [];
  for (let i = 0; i < tokens.length; ) {
    const raw = tokens[i];
    const w = raw.toLowerCase();
    let used = 0;
    const date = (at: number, set: (d: string) => void, extra: number) => {
      const r = dateAt(tokens, at, today);
      if (r) { set(r[0]); used = extra + r[1]; }
    };
    if (priority(w) !== null) { out.priority = priority(w); used = 1; }
    else if (w.length > 1 && w.startsWith('#')) { out.slug = w.slice(1); used = 1; }
    else if (raw.length > 1 && raw.startsWith('@')) { out.labels.push(raw.slice(1)); used = 1; }
    else if (estimate(w) !== null) { out.estimate = estimate(w); used = 1; }
    else if (pointsOn && points(w) !== null) { out.points = points(w); used = 1; }
    else if (pointsOn && pointsNumber(w) !== null && /^pts?$/i.test(tokens[i + 1] ?? '')) { out.points = pointsNumber(w); used = 2; }
    else if (w === 'by' || w === 'due') date(i + 1, (d) => (out.dueDate = d), 1);
    else if (w === 'start') date(i + 1, (d) => (out.start = d), 1);
    else if (w === 'at') {
      const t = timeAt(tokens[i + 1]);
      if (t) { out.time = [t, tokens[i + 1]]; used = 2; }
    } else date(i, (d) => (out.dueDate = d), 0);
    if (used === 0) { title.push(raw); i += 1; } else i += used;
  }
  let due: Due | null = out.dueDate ? { Date: out.dueDate } : null;
  if (out.time) {
    const d = toDue(out.dueDate ?? today, out.time[0], tz);
    if (d) due = d;
    else title.push('at', out.time[1]);
  }
  return {
    title: title.length ? title.join(' ') : tokens.join(' '), due, start_date: out.start, priority: out.priority,
    project_slug: out.slug, label_names: out.labels, estimate_minutes: out.estimate, points: out.points,
  };
}
