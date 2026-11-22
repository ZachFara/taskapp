import { api } from '../../lib/api';
import type { Measure } from './insightsApi';

export type Chart = 'burnup' | 'stacked' | 'bars';
export type Range = 'auto' | '30' | '90' | 'all';
export type Config = { measure: Measure; chart: Chart; range: Range };

const CHARTS: Chart[] = ['burnup', 'stacked', 'bars'];
const RANGES: Range[] = ['auto', '30', '90', 'all'];
const key = (kind: string, id: number) => `insights.${kind}.${id}`;

/** What the panel remembers for one view, project or cycle. Junk or a measure that is not allowed falls back. */
export async function loadConfig(kind: string, id: number, allowed: Measure[]): Promise<Config> {
  let stored: Partial<Config> = {};
  try {
    const raw = await api.settings.get(key(kind, id));
    stored = typeof raw === 'string' ? JSON.parse(raw) : {};
  } catch {
    // unreadable: use the defaults
  }
  return {
    measure: allowed.includes(stored.measure as Measure) ? (stored.measure as Measure) : allowed[0],
    chart: CHARTS.includes(stored.chart as Chart) ? (stored.chart as Chart) : 'burnup',
    range: RANGES.includes(stored.range as Range) ? (stored.range as Range) : 'auto',
  };
}

export const saveConfig = (kind: string, id: number, c: Config) => api.settings.set(key(kind, id), JSON.stringify(c)).catch(() => null);
