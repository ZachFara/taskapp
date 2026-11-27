// Port of crates/core/src/metrics.rs: delivery numbers replayed from the change history.
import type { Bucket, Change, Cycle, Issue, Label, Measure, Metrics, MetricsBucket, MetricsQuery, Status } from './bindings';
import { MAX_DAYS, addDays, localDay } from './api.mock.insights';

type Cat = Status['category'];
type Facts = { issue: Issue; made: string; madeBy: string; cat0: Cat; moves: [string, Cat][]; started: string | null; done: string | null; doneBy: string };
type Inputs = { changes: Change[]; statuses: Status[]; cycles: Cycle[]; labels: Label[]; tz: string; today: string };

const cmp = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
const gap = (from: string, to: string) => Math.max(0, Math.round((Date.parse(to) - Date.parse(from)) / 86_400_000));
const sum = (fs: Facts[], f: (x: Facts) => number) => fs.reduce((a, x) => a + f(x), 0);
const points = (f: Facts) => f.issue.points ?? 0;
const finishedIn = (f: Facts, a: string, b: string) => f.done !== null && a <= f.done && f.done <= b;

/** Nearest-rank percentile (`pct` out of 100). */
const percentile = (days: number[], pct: number) => [...days].sort((a, b) => a - b)[Math.max(1, Math.ceil((days.length * pct) / 100)) - 1] ?? null;

const bucketStart = (day: string, b: Bucket) =>
  b === 'Day' ? day : b === 'Week' ? addDays(day, -((new Date(`${day}T00:00:00Z`).getUTCDay() + 6) % 7)) : `${day.slice(0, 8)}01`;
const nextStart = (start: string, b: Bucket) => {
  if (b !== 'Month') return addDays(start, b === 'Day' ? 1 : 7);
  const d = new Date(`${start}T00:00:00Z`);
  d.setUTCMonth(d.getUTCMonth() + 1);
  return d.toISOString().slice(0, 10);
};

function story(issue: Issue, { changes, statuses, tz }: Inputs): Facts {
  const cats = new Map(statuses.map((s) => [s.id, s.category]));
  const rows = changes.filter((c) => c.entity === 'issue' && c.entity_id === String(issue.id) && (c.field === 'key' || c.field === 'status_id'))
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id - b.id);
  const events = rows.filter((c) => c.field === 'status_id');
  const catOf = (v: unknown) => (typeof v === 'number' ? cats.get(v) : undefined);
  const now = cats.get(issue.status_id);
  const moves = events.flatMap((c) => { const k = catOf(c.new_value); return k ? [[localDay(c.at, tz), k, c.source] as [string, Cat, string]] : []; });
  const finishing = [...moves].reverse().find((m) => m[1] === 'Completed');
  // The mock's seed has no creation rows (the real one does, from the UI), so a missing one reads as the UI.
  const madeBy = rows.find((c) => c.field === 'key')?.source ?? 'Ui';
  let [done, doneBy]: [string | null, string] = [null, ''];
  if (now === 'Completed') [done, doneBy] = finishing ? [finishing[0], finishing[2]] : [issue.completed_at ? localDay(issue.completed_at, tz) : null, 'Migration'];
  const first = moves.find((m) => m[1] === 'Started')?.[0] ?? null;
  return {
    issue, made: localDay(issue.created_at, tz), madeBy, cat0: catOf(events[0]?.old_value) ?? now ?? 'Backlog',
    moves: moves.map((m) => [m[0], m[1]]), started: first !== null && done !== null && first <= done ? first : null, done, doneBy,
  };
}

/** The category at the end of `day` and the day it was entered; null before the issue existed. */
function at(f: Facts, day: string): [Cat, string] | null {
  if (f.made > day) return null;
  const m = [...f.moves].reverse().find((x) => x[0] <= day);
  return m ? [m[1], m[0]] : [f.cat0, f.made];
}

function buckets(mine: Facts[], from: string, to: string, kind: Bucket): MetricsBucket[] {
  const out: MetricsBucket[] = [];
  for (let start = bucketStart(from, kind); start <= to; ) {
    const next = nextStart(start, kind);
    const [a, b] = [start > from ? start : from, addDays(next, -1) < to ? addDays(next, -1) : to];
    let [open, finished, wip] = [0, 0, 0];
    for (const state of mine.map((f) => at(f, b))) {
      if (!state) continue;
      if (state[0] === 'Completed' && state[1] >= from) finished++;
      else if (state[0] === 'Backlog' || state[0] === 'Unstarted') open++;
      else if (state[0] === 'Started') { open++; wip++; }
    }
    const done = mine.filter((f) => finishedIn(f, a, b));
    out.push({
      start, created: mine.filter((f) => a <= f.made && f.made <= b).length, completed: done.length,
      completed_points: sum(done, points), scope: open + finished, done_total: finished, wip,
    });
    start = next;
  }
  return out;
}

export function metrics(q: MetricsQuery, filtered: Issue[], inp: Inputs, fail: (kind: string, message: string) => never): Metrics {
  if (q.measure === 'time') fail('Invalid', 'metrics count issues or points, not time');
  let [from, to, cycle] = ['', '', undefined as Cycle | undefined];
  const range = q.scope.Range;
  if (range) [from, to] = [range.from, range.to];
  else {
    const id = q.scope.Cycle!;
    cycle = inp.cycles.find((c) => c.id === id) ?? fail('NotFound', `cycle ${id}`);
    [from, to] = [cycle.start_date, cycle.end_date];
  }
  if (to < from) fail('Invalid', `range ends (${to}) before it starts (${from})`);
  if ((Date.parse(to) - Date.parse(from)) / 86_400_000 >= MAX_DAYS) fail('Invalid', `range ${from} to ${to} must be 1 to ${MAX_DAYS} days`);
  const all = filtered.map((i) => story(i, inp));
  const mine = all.filter((f) => !cycle || f.issue.cycle_id === cycle!.id);
  const done = mine.filter((f) => finishedIn(f, from, to));

  const lead = done.map((f) => gap(f.made, f.done!));
  const timed = done.filter((f) => f.started).sort((a, b) => cmp(a.done!, b.done!));
  const days = timed.map((f) => gap(f.started!, f.done!));

  const names = new Map(inp.labels.map((l) => [l.id, l.name]));
  const tally = new Map<number | null, [number, number]>();
  for (const f of done) for (const id of f.issue.label_ids.length ? f.issue.label_ids : [null]) tally.set(id, [(tally.get(id)?.[0] ?? 0) + 1, (tally.get(id)?.[1] ?? 0) + points(f)]);
  const by_label = [...tally].sort((a, b) => (a[0] ?? -Infinity) - (b[0] ?? -Infinity)).map(([label_id, [completed, pts]]) => ({
    label_id, name: label_id === null ? 'No label' : (names.get(label_id) ?? 'No label'), completed, points: pts,
  })).sort((a, b) => (q.measure === 'points' ? b.points - a.points : b.completed - a.completed) || cmp(a.name, b.name));

  const sources = new Map<string, [number, number]>();
  const bump = (s: string, i: 0 | 1) => { const e = sources.get(s) ?? [0, 0]; e[i]++; sources.set(s, e); };
  for (const f of mine) { if (from <= f.made && f.made <= to) bump(f.madeBy, 0); if (finishedIn(f, from, to)) bump(f.doneBy, 1); }
  const by_source = [...sources].sort((a, b) => cmp(a[0], b[0])).map(([source, [created, completed]]) => ({ source, created, completed }))
    .sort((a, b) => b.created + b.completed - (a.created + a.completed) || cmp(a.source, b.source));

  const past = inp.cycles.filter((c) => c.end_date < inp.today).sort((a, b) => a.number - b.number).slice(-6);
  const velocity = past.map((c) => {
    const d = all.filter((f) => finishedIn(f, c.start_date, c.end_date));
    return { cycle_id: c.id, name: c.name ?? `Cycle ${c.number}`, points: sum(d, points), count: d.length };
  });
  return {
    from, to, bucket: q.bucket, buckets: buckets(mine, from, to, q.bucket),
    lead_time: { p50_days: percentile(lead, 50), p85_days: percentile(lead, 85), n: lead.length },
    cycle_time: {
      p50_days: percentile(days, 50), p85_days: percentile(days, 85), n: days.length,
      points: timed.map((f) => ({ issue_id: f.issue.id, key: f.issue.key, title: f.issue.title, completed: f.done!, days: gap(f.started!, f.done!) })),
    },
    wip_now: mine.filter((f) => at(f, inp.today)?.[0] === 'Started').length,
    by_label, by_source, velocity, thin: done.length < 5,
  };
}
