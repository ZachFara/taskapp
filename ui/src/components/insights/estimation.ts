import { api } from '../../lib/api';
import type { Issue } from '../../lib/bindings';
import type { Measure } from './insightsApi';

export type Estimation = 'none' | 'points' | 'time' | 'both';
export type ScaleName = 'fibonacci' | 'linear' | 'powers' | 'tshirt';

const nums = (...v: number[]) => v.map((n) => ({ value: n, label: String(n) }));
export const SCALES: Record<ScaleName, { value: number; label: string }[]> = {
  fibonacci: nums(0, 1, 2, 3, 5, 8, 13, 21),
  linear: nums(1, 2, 3, 4, 5, 6, 7, 8, 9, 10),
  powers: nums(1, 2, 4, 8, 16),
  tshirt: [['XS', 1], ['S', 2], ['M', 3], ['L', 5], ['XL', 8]].map(([label, value]) => ({ label: label as string, value: value as number })),
};

export const estimationItems = [
  { value: 'none', label: 'None' }, { value: 'points', label: 'Points' }, { value: 'time', label: 'Time' }, { value: 'both', label: 'Both' },
];
export const scaleItems = [
  { value: 'fibonacci', label: 'Fibonacci (0, 1, 2, 3, 5, 8, 13, 21)' }, { value: 'linear', label: 'Linear (1 to 10)' },
  { value: 'powers', label: 'Powers of two (1, 2, 4, 8, 16)' }, { value: 'tshirt', label: 'T-shirt (XS to XL)' },
];

export async function readEstimation(): Promise<{ estimation: Estimation; scale: ScaleName }> {
  const get = (k: string) => api.settings.get(k).catch(() => null);
  const [e, s] = await Promise.all([get('estimation'), get('point_scale')]);
  return {
    estimation: estimationItems.some((i) => i.value === e) ? (e as Estimation) : 'both',
    scale: s && s in SCALES ? (s as ScaleName) : 'fibonacci',
  };
}

export const pointsOn = (e: Estimation) => e === 'points' || e === 'both';
/** Only the measures the estimation setting allows; issue counts are always possible. */
export const measuresFor = (e: Estimation): Measure[] => [...(pointsOn(e) ? ['points' as const] : []), 'count', ...(e === 'time' || e === 'both' ? ['time' as const] : [])];

/** `points` is not in the generated Issue yet, so it is read through a guard. */
export const pointsOf = (i: Issue): number | null => {
  const p = (i as { points?: unknown }).points;
  return typeof p === 'number' ? p : null;
};

export const unitOf = (m: Measure) => (m === 'points' ? 'points' : m === 'time' ? 'hours' : 'issues');
export const formatFor = (m: Measure) => (m === 'time' ? (n: number) => `${Math.round((n / 60) * 10) / 10}h` : (n: number) => String(Math.round(n * 10) / 10));

/** The measure a chart uses when nobody chose one: points when the workspace estimates them, else issue counts. */
export const defaultMeasure = async (): Promise<Measure> => measuresFor((await readEstimation()).estimation)[0];
/** The "Progress charts" setting; on unless it was switched off. */
export const chartsOn = async () => (await api.settings.get('insights_charts').catch(() => null)) !== false;
