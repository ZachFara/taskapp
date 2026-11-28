import type { Goal } from './bindings';

/** The quarter a date falls in, like `2027-Q1`. */
export const quarterOf = (d = new Date()) => `${d.getFullYear()}-Q${Math.floor(d.getMonth() / 3) + 1}`;

export const STATUS_LABEL: Record<Goal['status'], string> = { active: 'Active', achieved: 'Achieved', dropped: 'Dropped' };

/** Goals grouped by quarter, newest quarter first. */
export function byQuarter(goals: Goal[]): [string, Goal[]][] {
  const groups = new Map<string, Goal[]>();
  for (const g of goals) groups.set(g.quarter, [...(groups.get(g.quarter) ?? []), g]);
  return [...groups].sort((a, b) => b[0].localeCompare(a[0]));
}
