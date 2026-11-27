// TEMPORARY (DASH): a stand-in for the engine's `metrics.for`, built on the insights timelines. The orchestrator replaces
// this file and its hook in api.mock.ts with the engine builder's version at merge. Counts only; points are summed from issues.
import type { Change, Cycle, Issue, Status } from './bindings';
import type { Metrics, MetricsQuery } from '../components/dashboard/metrics';
import { addDays, timelines, totals } from './api.mock.insights';

const pct = (xs: number[], p: number) => (xs.length ? [...xs].sort((a, b) => a - b)[Math.max(0, Math.ceil(p * xs.length) - 1)] : null);
const gap = (a: string, b: string) => Math.max(0, (Date.parse(b) - Date.parse(a)) / 86_400_000);

export function metricsFor(q: MetricsQuery, s: { issues: Issue[]; statuses: Status[]; changes: Change[]; cycles: Cycle[]; tz: string; match: (i: Issue) => boolean }): Metrics {
  const [from, to] = 'Cycle' in q.scope ? (({ start_date, end_date }) => [start_date, end_date])(s.cycles.find((c) => c.id === (q.scope as { Cycle: number }).Cycle)!) : [q.scope.Range.from, q.scope.Range.to];
  const rows = s.issues.filter((i) => !i.archived_at && s.match(i) && (!('Cycle' in q.scope) || i.cycle_id === q.scope.Cycle));
  const tls = timelines(rows, s.changes, s.statuses, 'count', s.tz);
  const facts = tls.map((t, k) => {
    const done = [...t.cat].reverse().find(([, c]) => c === 'Completed')?.[0] ?? (t.cat0 === 'Completed' ? t.created : null);
    const began = t.cat.find(([, c]) => c === 'Started')?.[0] ?? (t.cat0 === 'Started' ? t.created : null);
    return { t, done: t.cat.at(-1)?.[1] === 'Completed' || (!t.cat.length && t.cat0 === 'Completed') ? done : null, began, issue: rows[k] };
  });
  const starts: string[] = [];
  for (let d = from; d <= to; d = q.bucket === 'Day' ? addDays(d, 1) : q.bucket === 'Week' ? addDays(d, 7) : new Date(Date.UTC(+d.slice(0, 4), +d.slice(5, 7), 1)).toISOString().slice(0, 10)) starts.push(d);
  const next = (i: number) => (i + 1 < starts.length ? addDays(starts[i + 1], -1) : to);
  const within = (d: string | null, i: number) => !!d && d >= starts[i] && d <= next(i);
  const buckets = starts.map((start, i) => {
    const [scope, , done] = totals(tls, next(i));
    const t = totals(tls, next(i));
    return {
      start, scope, done_total: done, wip: t[1] - t[2],
      created: facts.filter((f) => within(f.t.created, i)).length,
      completed: facts.filter((f) => within(f.done, i)).length,
      completed_points: facts.filter((f) => within(f.done, i)).reduce((n, f) => n + (f.issue.points ?? 0), 0),
    };
  });
  const finished = facts.filter((f) => f.done && f.done >= from && f.done <= to);
  const cycle = finished.filter((f) => f.began).map((f) => ({ issue_id: f.issue.id, key: f.issue.key, title: f.issue.title, completed: f.done!, days: gap(f.began!, f.done!) }));
  const lead = finished.map((f) => gap(f.t.created, f.done!));
  return {
    from, to, bucket: q.bucket, buckets,
    lead_time: { p50_days: pct(lead, 0.5), p85_days: pct(lead, 0.85), n: lead.length },
    cycle_time: { p50_days: pct(cycle.map((c) => c.days), 0.5), p85_days: pct(cycle.map((c) => c.days), 0.85), n: cycle.length, points: cycle },
    wip_now: rows.filter((i) => s.statuses.find((x) => x.id === i.status_id)?.category === 'Started').length,
    by_label: [], by_source: [], velocity: [], thin: finished.length < 5,
  };
}
