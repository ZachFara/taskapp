/** Repeat rules, a port of crates/core/src/recur.rs: same text forms, same canonical text, same date math. */

type Unit = 'day' | 'week' | 'month';
export type Rule = { n: number; unit: Unit; days: number[]; monthDay: number | null; fromDone: boolean };

const NAMES = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'];
const LONG = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const WEEKDAYS = [0, 1, 2, 3, 4];
const UNITS: Record<string, Unit> = {
  day: 'day', days: 'day', daily: 'day', week: 'week', weeks: 'week', weekly: 'week', weekdays: 'week', weekday: 'week',
  month: 'month', months: 'month', monthly: 'month',
};

/** The rule for a text like "every monday" or "every 2 weeks from completion", or null when it is not one. */
export function parseRule(text: string): Rule | null {
  let toks = text.toLowerCase().replace(/,/g, ' ').split(/\s+/).filter((t) => t && t !== 'and' && t !== 'the');
  let fromDone = false;
  const at = toks.findIndex((t) => t === 'from' || t === 'after');
  if (at >= 0) {
    const w = toks[at + 1];
    if (w === 'due') fromDone = false;
    else if (['completion', 'complete', 'completed', 'done'].includes(w)) fromDone = true;
    else return null;
    toks = [...toks.slice(0, at), ...toks.slice(at + 2)];
  }
  if (toks[0] === 'every') toks.shift();
  const head = toks[0];
  if (head === undefined) return null;
  let n = 1;
  let days: number[] = head.startsWith('weekday') ? [...WEEKDAYS] : [];
  const dayOf = (t: string) => NAMES.indexOf(t.slice(0, 3));
  let unit: Unit | undefined;
  if (/^\d+$/.test(head)) {
    n = Number(head);
    unit = UNITS[toks[1]];
    toks = toks.slice(2);
  } else if (UNITS[head]) {
    unit = UNITS[head];
    toks = toks.slice(1);
  } else if (dayOf(head) >= 0 && head.length >= 3) unit = 'week';
  if (!unit) return null;
  if (toks[0] === 'on') toks.shift();
  if (toks[0] === 'day') toks.shift();
  let monthDay: number | null = null;
  for (const t of toks) {
    if (t.length >= 3 && dayOf(t) >= 0) days.push(dayOf(t));
    else if (/^\d+/.test(t) && /^\d+[a-z]*$/.test(t)) monthDay = parseInt(t, 10);
    else return null;
  }
  days = [...new Set(days)].sort((a, b) => a - b);
  const fits = unit === 'day' ? !days.length && monthDay === null : unit === 'week' ? monthDay === null : !days.length && (monthDay === null || (monthDay >= 1 && monthDay <= 31));
  return fits && n >= 1 && n <= 365 ? { n, unit, days, monthDay, fromDone } : null;
}

/** The canonical text the backend stores. */
export function ruleText(r: Rule): string {
  const on = r.days.length ? ` on ${r.days.map((d) => NAMES[d]).join(',')}` : r.monthDay !== null ? ` on ${r.monthDay}` : '';
  return `every ${r.n} ${r.unit}${on} from ${r.fromDone ? 'done' : 'due'}`;
}

export const canonical = (text: string) => { const r = parseRule(text); return r && ruleText(r); };

/** The rule in plain words. */
export function ruleWords(r: Rule): string {
  const weekdays = r.days.length === 5 && WEEKDAYS.every((d) => r.days.includes(d));
  let out = r.n === 1 ? (weekdays ? 'Every weekday' : `Every ${r.unit}`) : `Every ${r.n} ${r.unit}s`;
  if (r.days.length && !weekdays) out += ` on ${r.days.map((d) => LONG[d]).join(' and ')}`;
  if (r.monthDay !== null) out += ` on day ${r.monthDay}`;
  return out + (r.fromDone ? ', from completion' : '');
}

const parts = (d: string) => d.split('-').map(Number) as [number, number, number];
const iso = (y: number, m: number, day: number) => new Date(Date.UTC(y, m - 1, day)).toISOString().slice(0, 10);
const addDays = (d: string, n: number) => { const [y, m, day] = parts(d); return iso(y, m, day + n); };
const monthEnd = (y: number, m: number) => new Date(Date.UTC(y, m, 0)).getUTCDate();
/** Monday-first weekday, 0 to 6. */
const weekdayOf = (d: string) => (new Date(`${d}T00:00:00Z`).getUTCDay() + 6) % 7;
/** `d` in `months` months, on `day` (default the same day), clamped to the month's end. */
const addMonths = (d: string, months: number, day?: number) => {
  const [y, m, dd] = parts(d);
  const t = new Date(Date.UTC(y, m - 1 + months, 1));
  const [ny, nm] = [t.getUTCFullYear(), t.getUTCMonth() + 1];
  return iso(ny, nm, Math.min(day ?? dd, monthEnd(ny, nm)));
};

function step(r: Rule, d: string): string {
  if (r.unit === 'day') return addDays(d, r.n);
  if (r.unit === 'week') {
    if (!r.days.length) return addDays(d, 7 * r.n);
    const w = weekdayOf(d);
    const later = r.days.find((x) => x > w);
    return later !== undefined ? addDays(d, later - w) : addDays(d, -w + 7 * r.n + r.days[0]);
  }
  if (r.monthDay === null) return addMonths(d, r.n);
  const this_ = addMonths(d, 0, r.monthDay);
  return this_ > d ? this_ : addMonths(d, r.n, r.monthDay);
}

/** The next due date; `base` is the old due day (today when none), `today` the completion day. */
export function nextDue(r: Rule, base: string, today: string): string {
  if (r.fromDone) return step(r, today);
  let d = step(r, base);
  while (d <= today) d = step(r, d);
  return d;
}

/** Ticked task boxes unticked outside code fences, as core's checklist::reset. */
export function resetChecklist(text: string): string {
  let fenced = false;
  return text.split('\n').map((line) => {
    const t = line.trimStart();
    if (t.startsWith('```') || t.startsWith('~~~')) { fenced = !fenced; return line; }
    return !fenced && /^[-*+]\s+\[[xX]\](\s|$)/.test(t) ? line.replace(/\[[xX]\]/, '[ ]') : line;
  }).join('\n');
}
