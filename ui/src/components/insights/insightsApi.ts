import { api } from '../../lib/api';
import type { Day } from '../charts/geometry';

// The shapes the backend returns for the progress and velocity queries. They are declared here until the generated
// bindings carry them; every read is defensive because the backend may not have the commands yet.
export type Measure = 'points' | 'count' | 'time';
export type Scope = { Project: number } | { Cycle: number } | { View: number };
export type ProgressSeries = { days: Day[]; measure: Measure; from: string; to: string };
export type Period = { start: string; end: string; label: string; completed: number };
type Source = {
  progress: (scope: Scope, measure: Measure, from?: string, to?: string) => Promise<ProgressSeries | null>;
  velocity: (scope: Scope, measure: Measure, periods: number) => Promise<Period[]>;
};

let injected: Source | null = null;
const backend = () => (api as unknown as { insights?: { progress?: unknown; velocity?: unknown } }).insights;

const source = (): Source | null => {
  if (injected) return injected;
  const b = backend();
  if (typeof b?.progress !== 'function' || typeof b?.velocity !== 'function') return null;
  const call = <T>(name: 'progress' | 'velocity', args: unknown[]) => (b as Record<string, (...a: unknown[]) => Promise<T>>)[name](...args);
  return {
    progress: (...a) => call('progress', a),
    velocity: (...a) => call('velocity', a),
  };
};

/** Progress and velocity, or "nothing" (null, []) when the backend has none or fails: a chart is never worth an error. */
export const insightsApi = {
  /** Tests inject data; pass null to go back to the real api. */
  setSource(s: Source | null) {
    injected = s;
  },
  available: () => source() !== null,
  progress: (scope: Scope, measure: Measure, from?: string, to?: string) =>
    source()?.progress(scope, measure, from, to).catch(() => null) ?? Promise.resolve(null),
  velocity: (scope: Scope, measure: Measure, periods = 8) =>
    source()?.velocity(scope, measure, periods).catch(() => []) ?? Promise.resolve([] as Period[]),
};
