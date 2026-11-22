// Port of crates/core/src/insights.rs: progress rebuilt from the change history, one local day at a time.
import type { Change, Issue, Measure, ProgressSeries, Status, VelocityPeriod } from './bindings';

type Cat = Status['category'];
type Timeline = { created: string; cat0: Cat; cat: [string, Cat][]; value0: number; value: [string, number][] };

export const MAX_DAYS = 1100;
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
export const addDays = (date: string, n: number) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + n);
  return d.toISOString().slice(0, 10);
};
export const localDay = (iso: string, tz: string) => new Date(iso).toLocaleDateString('sv', { timeZone: tz });

const latest = <T>(initial: T, events: [string, T][], day: string): T => [...events].reverse().find(([d]) => d <= day)?.[1] ?? initial;
const num = (v: unknown) => (typeof v === 'number' ? v : 0);

export function timelines(issues: Issue[], changes: Change[], statuses: Status[], measure: Measure, tz: string): Timeline[] {
  const cat = (id: number): Cat => statuses.find((s) => s.id === id)?.category ?? 'Backlog';
  const field = { points: 'points', time: 'estimate_minutes', count: '' }[measure];
  const mine = changes
    .filter((c) => c.entity === 'issue' && (c.field === 'status_id' || c.field === field))
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id - b.id);
  return issues.map((i) => {
    const of = (f: string) => mine.filter((c) => c.entity_id === String(i.id) && c.field === f);
    const [status, value] = [of('status_id'), of(field)];
    const current = measure === 'points' ? (i.points ?? 0) : measure === 'time' ? (i.estimate_minutes ?? 0) : 1;
    return {
      created: localDay(i.created_at, tz),
      cat0: cat(typeof status[0]?.old_value === 'number' ? status[0].old_value : i.status_id),
      cat: status.flatMap((c) => (typeof c.new_value === 'number' ? [[localDay(c.at, tz), cat(c.new_value)] as [string, Cat]] : [])),
      value0: value.length ? num(value[0].old_value) : current,
      value: value.map((c) => [localDay(c.at, tz), num(c.new_value)] as [string, number]),
    };
  });
}

function at(t: Timeline, day: string): [number, number, number] {
  if (day < t.created) return [0, 0, 0];
  const v = latest(t.value0, t.value, day);
  switch (latest(t.cat0, t.cat, day)) {
    case 'Canceled': return [0, 0, 0];
    case 'Completed': return [v, v, v];
    case 'Started': return [v, v, 0];
    default: return [v, 0, 0];
  }
}
export const totals = (tls: Timeline[], day: string): [number, number, number] =>
  tls.reduce<[number, number, number]>((a, t) => { const b = at(t, day); return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]; }, [0, 0, 0]);

/** First creation to last status or value change; today when there are no issues. */
export function activity(tls: Timeline[], today: string): [string, string] {
  if (!tls.length) return [today, today];
  const days = tls.flatMap((t) => [t.created, ...t.cat.map((e) => e[0]), ...t.value.map((e) => e[0])]);
  return [tls.map((t) => t.created).sort()[0], days.sort().at(-1)!];
}

export function series(tls: Timeline[], measure: Measure, from: string, to: string): ProgressSeries {
  const days = [];
  for (let date = from; date <= to; date = addDays(date, 1)) {
    const [scope, started, completed] = totals(tls, date);
    days.push({ date, scope, started, completed });
  }
  return { days, measure, from, to };
}

/** Weeks start on Monday. Spans are cycles that have started (last `n`), else the last `n` weeks. */
export function velocity(tls: Timeline[], spans: { start: string; end: string; label: string }[]): VelocityPeriod[] {
  return spans.map(({ start, end, label }) => ({ start, end, label, completed: Math.max(0, totals(tls, end)[2] - totals(tls, addDays(start, -1))[2]) }));
}

export function weekSpans(today: string, n: number) {
  const monday = addDays(today, -((new Date(`${today}T00:00:00Z`).getUTCDay() + 6) % 7));
  return Array.from({ length: n }, (_, k) => {
    const start = addDays(monday, -7 * (n - 1 - k));
    const [, m, d] = start.split('-').map(Number);
    return { start, end: addDays(start, 6), label: `Week of ${MONTHS[m - 1]} ${d}` };
  });
}
