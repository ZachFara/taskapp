import { api } from '../../lib/api';
import type { Cycle, Filter } from '../../lib/bindings';
import { addDays } from '../../lib/api.mock.insights';
import type { Day } from '../charts/geometry';

// A local mirror of the engine's shape (.scratch/p5-briefs/METRICS-SHAPE.md) until bindings.ts has it; then import from there.
export type Bucket = 'Day' | 'Week' | 'Month';
export type Pct = { p50_days: number | null; p85_days: number | null; n: number };
export type Scope = { Range: { from: string; to: string } } | { Cycle: number };
export type MetricsQuery = { scope: Scope; bucket: Bucket; filter: Filter | null; measure: 'Count' | 'Points' };
export type Metrics = {
  from: string; to: string; bucket: Bucket;
  buckets: { start: string; created: number; completed: number; completed_points: number; scope: number; done_total: number; wip: number }[];
  lead_time: Pct;
  cycle_time: Pct & { points: { issue_id: number; key: string; title: string; completed: string; days: number }[] };
  wip_now: number;
  by_label: { label_id: number | null; name: string; completed: number; points: number }[];
  by_source: { source: string; created: number; completed: number }[];
  velocity: { cycle_id: number; name: string; points: number; count: number }[];
  thin: boolean;
  // Added by MIT-332/333/334, all optional: older engines send none of them. `cycle` is present for a cycle scope only.
  cycle?: CycleCommit | null;
  blocked?: Blocked | null;
  flow?: FlowEfficiency | null;
  /** Deep focus hours per bucket; absent from an engine that predates the focus feature. */
  deep?: DeepHours | null;
};
export type DeepHours = { buckets: { start: string; hours: number | null }[]; total_hours: number | null; streak_days: number };
export type CycleCommit = {
  committed: number; committed_points: number; done: number; done_points: number; say_do_pct: number | null; say_do_points_pct: number | null;
  added: number; added_points: number; removed: number; removed_points: number; scope_change_pct: number | null; scope_change_points_pct: number | null;
};
export type Blocked = {
  now: { issue_id: number; key: string; title: string; blocked_by: string[]; days: number }[];
  time: { issue_id: number; key: string; title: string; days: number }[];
  avg_days: number | null;
};
export type FlowEfficiency = { avg_pct: number | null; n: number; distribution: number[]; waiting: { status_id: number; name: string; days: number; issues: number }[] };

/** A percentage to the nearest whole number, or a dash. */
export const pct = (n: number | null | undefined) => (n == null ? '–' : `${Math.round(n)}%`);
/** Days to one decimal, or a dash. */
export const days = (n: number | null | undefined) => (n == null ? '–' : `${Math.round(n * 10) / 10} d`);

// The engine's measure is the shared lowercase Measure type; the page's own copy of the shape stays until bindings replace it.
type Wire = { metrics: { for: (q: Omit<MetricsQuery, 'measure'> & { measure: 'count' | 'points' }) => Promise<Metrics> } };
export const metricsFor = (q: MetricsQuery) =>
  (api as unknown as Wire).metrics.for({ ...q, measure: q.measure === 'Points' ? 'points' : 'count' });

export const PRESETS = [
  { id: 'today', label: 'Today' }, { id: 'week', label: 'This week' }, { id: 'cycle', label: 'This cycle' },
  { id: 'month', label: 'Month' }, { id: 'quarter', label: 'Quarter' }, { id: 'year', label: 'Year' }, { id: 'custom', label: 'Custom' },
];
export type RangeCtx = { today: string; weekStart: 'sunday' | 'monday'; cycles: Pick<Cycle, 'id' | 'start_date' | 'end_date'>[] };
export type Range = { scope: Scope; from: string; to: string };

const monthEnd = (y: number, m: number) => new Date(Date.UTC(y, m + 1, 0)).toISOString().slice(0, 10);
const first = (y: number, m: number) => new Date(Date.UTC(y, m, 1)).toISOString().slice(0, 10);

/** The days a preset covers, or null when it has none (no cycle today, a custom range not filled in). `cycle:<id>` is any cycle. */
export function rangeOf(preset: string, c: RangeCtx, from = '', to = ''): Range | null {
  const range = (f: string, t: string): Range => ({ scope: { Range: { from: f, to: t } }, from: f, to: t });
  const [y, m] = [Number(c.today.slice(0, 4)), Number(c.today.slice(5, 7)) - 1];
  if (preset === 'today') return range(c.today, c.today);
  if (preset === 'week') {
    const back = (new Date(`${c.today}T00:00:00Z`).getUTCDay() - (c.weekStart === 'monday' ? 1 : 0) + 7) % 7;
    return range(addDays(c.today, -back), addDays(c.today, 6 - back));
  }
  if (preset === 'month') return range(first(y, m), monthEnd(y, m));
  if (preset === 'quarter') return range(first(y, m - (m % 3)), monthEnd(y, m - (m % 3) + 2));
  if (preset === 'year') return range(`${y}-01-01`, `${y}-12-31`);
  if (preset === 'custom') return from && to ? range(from < to ? from : to, from < to ? to : from) : null;
  const cycle = preset === 'cycle' ? c.cycles.find((x) => x.start_date <= c.today && c.today <= x.end_date) : c.cycles.find((x) => `cycle:${x.id}` === preset);
  return cycle ? { scope: { Cycle: cycle.id }, from: cycle.start_date, to: cycle.end_date } : null;
}

/** Day up to 31 days, Week up to about six months, Month beyond. */
export function bucketFor(from: string, to: string): Bucket {
  const days = (Date.parse(to) - Date.parse(from)) / 86_400_000 + 1;
  return days <= 31 ? 'Day' : days <= 186 ? 'Week' : 'Month';
}

export function filterOf(project: number | null, label: number | null): Filter | null {
  const cond = (f: 'Project' | 'Label', value: number): Filter => ({ Cond: { field: { Fixed: f }, op: 'Eq', value } });
  const parts = [...(project === null ? [] : [cond('Project', project)]), ...(label === null ? [] : [cond('Label', label)])];
  return parts.length < 2 ? (parts[0] ?? null) : { And: parts };
}

/** The burn-up chart's days from the buckets. Started repeats done: the compact chart does not draw it. */
export const burnupDays = (m: Metrics): Day[] => m.buckets.map((b) => ({ date: b.start, scope: b.scope, started: b.done_total, completed: b.done_total }));
