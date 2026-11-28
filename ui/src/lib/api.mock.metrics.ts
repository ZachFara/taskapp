// Port of crates/core/src/metrics.rs: delivery numbers replayed from the change history.
import type { Bucket, Change, Cycle, Forecast, ForecastQuery, Issue, Label, Measure, Metrics, MetricsBucket, MetricsQuery, Relation, Status } from './bindings';
import { MAX_DAYS, addDays, localDay } from './api.mock.insights';

type Cat = Status['category'];
type Facts = {
  issue: Issue; made: string; madeBy: string; cat0: Cat; moves: [string, Cat][]; started: string | null; done: string | null; doneBy: string;
  status0: number; path: [string, number][]; cycle0: number | null; cycles: [string, number | null][];
};
type Inputs = { changes: Change[]; statuses: Status[]; cycles: Cycle[]; labels: Label[]; relations: Relation[]; everyone: Issue[]; tz: string; today: string };

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
  const idOf = (v: unknown) => (typeof v === 'number' ? v : null);
  const cyc = changes.filter((c) => c.entity === 'issue' && c.entity_id === String(issue.id) && c.field === 'cycle_id').sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id - b.id);
  const first = moves.find((m) => m[1] === 'Started')?.[0] ?? null;
  return {
    issue, made: localDay(issue.created_at, tz), madeBy, cat0: catOf(events[0]?.old_value) ?? now ?? 'Backlog',
    status0: idOf(events[0]?.old_value) ?? issue.status_id,
    path: events.flatMap((c) => (typeof c.new_value === 'number' ? [[localDay(c.at, tz), c.new_value] as [string, number]] : [])),
    cycle0: cyc.length ? idOf(cyc[0].old_value) : issue.cycle_id, cycles: cyc.map((c) => [localDay(c.at, tz), idOf(c.new_value)]),
    moves: moves.map((m) => [m[0], m[1]]), started: first !== null && done !== null && first <= done ? first : null, done, doneBy,
  };
}

/** The category at the end of `day` and the day it was entered; null before the issue existed. */
function at(f: Facts, day: string): [Cat, string] | null {
  if (f.made > day) return null;
  const m = [...f.moves].reverse().find((x) => x[0] <= day);
  return m ? [m[1], m[0]] : [f.cat0, f.made];
}

const unfinished = (f: Facts | undefined, day: string) => ['Backlog', 'Unstarted', 'Started'].includes(at(f!, day)?.[0] ?? '');
const pct = (part: number, of: number) => (of > 0 ? (part * 100) / of : null);
const cycleAt = (f: Facts, day: string) => {
  const m = [...f.cycles].reverse().find((x) => x[0] <= day);
  return m ? m[1] : f.cycle0;
};

/** Members at the end of the start day against the end (or today) and what they finished. */
function commit(all: Facts[], c: Cycle, today: string) {
  let end = c.end_date < today ? c.end_date : today;
  if (end < c.start_date) end = c.start_date;
  const member = (f: Facts, day: string) => f.made <= day && cycleAt(f, day) === c.id;
  const [committed, added, removed] = [all.filter((f) => member(f, c.start_date)), all.filter((f) => member(f, end) && !member(f, c.start_date)), all.filter((f) => member(f, c.start_date) && !member(f, end))];
  const done = committed.filter((f) => at(f, end)?.[0] === 'Completed');
  const [cp, ch, dp] = [sum(committed, points), committed.length, sum(done, points)];
  return {
    committed: ch, committed_points: cp, done: done.length, done_points: dp, say_do_pct: pct(done.length, ch), say_do_points_pct: pct(dp, cp),
    added: added.length, added_points: sum(added, points), removed: removed.length, removed_points: sum(removed, points),
    scope_change_pct: pct(added.length + removed.length, ch), scope_change_points_pct: pct(sum(added, points) + sum(removed, points), cp),
  };
}

type Link = { blocker: number; from: string; until: string | null };
/** The `blocks` links into each issue: from the relation history, plus current relations that have none. */
function links(inp: Inputs, every: Facts[]): Map<number, Link[]> {
  const out = new Map<number, Link[]>();
  const rows = inp.changes.filter((c) => c.entity === 'issue' && c.field === 'relation').sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id - b.id);
  for (const c of rows) {
    const added = c.new_value !== null;
    const rel = (added ? c.new_value : c.old_value) as unknown as Relation;
    if (rel.kind !== 'blocks') continue;
    const mine = out.get(rel.to) ?? [];
    out.set(rel.to, mine);
    const day = localDay(c.at, inp.tz);
    if (added) mine.push({ blocker: rel.from, from: day, until: null });
    else { const l = [...mine].reverse().find((x) => x.blocker === rel.from && x.until === null); if (l) l.until = day; }
  }
  const now = inp.relations.filter((x) => x.kind === 'blocks');
  // The table is the truth: a link the history leaves open but the table lacks (an undo, say) never lasted.
  for (const [target, mine] of out) for (const l of mine) if (l.until === null && !now.some((r) => r.from === l.blocker && r.to === target)) l.until = l.from;
  const made = new Map(every.map((f) => [f.issue.id, f.made]));
  for (const r of now) {
    const mine = out.get(r.to) ?? [];
    out.set(r.to, mine);
    if (!mine.some((l) => l.blocker === r.from && l.until === null)) {
      const [a, b] = [made.get(r.from) ?? '', made.get(r.to) ?? ''];
      mine.push({ blocker: r.from, from: a > b ? a : b, until: null });
    }
  }
  return out;
}

function blocked(mine: Facts[], done: Facts[], every: Facts[], inp: Inputs) {
  const ls = links(inp, every);
  const byId = new Map(every.map((f) => [f.issue.id, f]));
  const order = <T extends { days: number; key: string }>(a: T, b: T) => b.days - a.days || cmp(a.key, b.key);
  const now = mine.filter((f) => unfinished(f, inp.today)).flatMap((f) => {
    const live = (ls.get(f.issue.id) ?? []).filter((l) => l.until === null && unfinished(byId.get(l.blocker), inp.today));
    if (!live.length) return [];
    const since = live.map((l) => l.from).reduce((a, b) => (a > b ? a : b));
    return [{ issue_id: f.issue.id, key: f.issue.key, title: f.issue.title, blocked_by: live.map((l) => byId.get(l.blocker)!.issue.key).sort(), days: gap(since, inp.today) }];
  }).sort(order);
  const time = done.flatMap((f) => {
    let days = 0;
    for (let d = f.made; d < (f.done ?? f.made); d = addDays(d, 1)) {
      if ((ls.get(f.issue.id) ?? []).some((l) => l.from <= d && (l.until === null || d < l.until) && unfinished(byId.get(l.blocker), d))) days++;
    }
    return days ? [{ issue_id: f.issue.id, key: f.issue.key, title: f.issue.title, days }] : [];
  }).sort(order);
  return { now, time, avg_days: done.length ? time.reduce((a, t) => a + t.days, 0) / done.length : null };
}

/** Active-kind days over cycle-time days per finished issue, and the Waiting statuses where work sat longest. */
function flow(done: Facts[], statuses: Status[]) {
  const by = new Map(statuses.map((s) => [s.id, s]));
  const [effs, waiting] = [[] as number[], new Map<number, [number, number]>()];
  for (const f of done) {
    if (!f.started || !f.done || f.done <= f.started) continue;
    const spent = new Map<number, number>();
    const line: [string, number][] = [[f.made, f.status0], ...f.path];
    line.forEach(([day, id], i) => {
      const [lo, hi] = [day > f.started! ? day : f.started!, (line[i + 1]?.[0] ?? f.done!) < f.done! ? (line[i + 1]?.[0] ?? f.done!) : f.done!];
      if (hi > lo) spent.set(id, (spent.get(id) ?? 0) + gap(lo, hi));
    });
    let active = 0;
    for (const [id, d] of spent) {
      const kind = by.get(id)?.flow_kind;
      if (kind === 'Active') active += d;
      if (kind === 'Waiting') { const e = waiting.get(id) ?? [0, 0]; waiting.set(id, [e[0] + d, e[1] + 1]); }
    }
    effs.push((active * 100) / gap(f.started, f.done));
  }
  const distribution = [0, 0, 0, 0, 0];
  for (const e of effs) distribution[[20, 40, 60, 80].filter((t) => e >= t).length]++;
  return {
    avg_pct: effs.length ? effs.reduce((a, b) => a + b, 0) / effs.length : null, n: effs.length, distribution,
    waiting: [...waiting].flatMap(([id, [days, issues]]) => (by.get(id) ? [{ status_id: id, name: by.get(id)!.name, days, issues }] : [])).sort((a, b) => b.days - a.days || cmp(a.name, b.name)),
  };
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
  // A blocker may fall outside the filter, so blockers are looked up among every issue.
  const every = q.filter ? inp.everyone.map((i) => story(i, inp)) : all;
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
    cycle: cycle ? commit(all, cycle, inp.today) : null,
    blocked: blocked(mine, done, every, inp), flow: flow(done, inp.statuses),
    wip_now: mine.filter((f) => at(f, inp.today)?.[0] === 'Started').length,
    by_label, by_source, velocity, thin: done.length < 5,
  };
}

// Port of crates/core/src/forecast.rs, mulberry32 and all, so the contract cases agree with the engine.
const WINDOW = 56;
const RUNS = 1000;
function mulberry(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), a | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return (t ^ (t >>> 14)) >>> 0;
  };
}

export function forecast(q: ForecastQuery, filtered: Issue[], inp: Inputs, fail: (kind: string, message: string) => never): Forecast {
  if (q.cycle !== null && !inp.cycles.some((c) => c.id === q.cycle)) fail('NotFound', `cycle ${q.cycle}`);
  const all = filtered.map((i) => story(i, inp));
  const perDay = new Array<number>(WINDOW).fill(0);
  for (const f of all) {
    const age = f.done === null ? -1 : gap(f.done, inp.today);
    if (f.done !== null && f.done <= inp.today && age < WINDOW) perDay[age]++;
  }
  const completions = perDay.reduce((a, b) => a + b, 0);
  const open = all.filter((f) => (q.cycle === null || f.issue.cycle_id === q.cycle) && ['Backlog', 'Unstarted', 'Started'].includes(at(f, inp.today)?.[0] ?? '')).length;
  const enough = completions >= 10;
  const runs: number[] = [];
  if (enough && open > 0) {
    const next = mulberry(0x00c0ffee);
    for (let r = 0; r < RUNS; r++) {
      let [left, days] = [open, 0];
      while (left > 0 && days < 3650) { left = Math.max(0, left - perDay[Math.floor((next() * WINDOW) / 2 ** 32)]); days++; }
      runs.push(days);
    }
    runs.sort((a, b) => a - b);
  }
  const on = (pct: number) => (runs.length ? addDays(inp.today, runs[Math.max(1, Math.ceil((runs.length * pct) / 100)) - 1]) : null);
  return { open, completions, window_days: WINDOW, enough, p50: on(50), p85: on(85) };
}
