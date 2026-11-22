import { api } from '../../lib/api';
import type { Day } from '../charts/geometry';

export type { Measure, ProgressScope as Scope } from '../../lib/bindings';
import type { Measure, ProgressScope as Scope } from '../../lib/bindings';
export type ProgressSeries = { days: Day[]; measure: Measure; from: string; to: string };
export type Period = { start: string; end: string; label: string; completed: number };
type Source = {
  progress: (scope: Scope, measure: Measure, from?: string, to?: string) => Promise<ProgressSeries | null>;
  velocity: (scope: Scope, measure: Measure, periods: number) => Promise<Period[]>;
};

let injected: Source | null = null;

// The generated api, with null totals (a day before the scope existed) read as zero.
const real: Source = {
  progress: async (scope, measure, from, to) => {
    const p = await api.insights.progress(scope, measure, from ?? null, to ?? null);
    return { ...p, days: p.days.map((d) => ({ date: d.date, scope: d.scope ?? 0, started: d.started ?? 0, completed: d.completed ?? 0 })) };
  },
  velocity: async (scope, measure, periods) =>
    (await api.insights.velocity(scope, measure, periods)).map((v) => ({ ...v, completed: v.completed ?? 0 })),
};
const source = (): Source => injected ?? real;

/** Progress and velocity, or "nothing" (null, []) when the backend has none or fails: a chart is never worth an error. */
export const insightsApi = {
  /** Tests inject data; pass null to go back to the real api. */
  setSource(s: Source | null) {
    injected = s;
  },
  progress: (scope: Scope, measure: Measure, from?: string, to?: string) =>
    source().progress(scope, measure, from, to).catch(() => null),
  velocity: (scope: Scope, measure: Measure, periods = 8) =>
    source().velocity(scope, measure, periods).catch(() => [] as Period[]),
};
