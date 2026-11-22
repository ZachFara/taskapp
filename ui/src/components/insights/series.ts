import { localDay } from '../../lib/gantt';
import { workspaceZone } from '../views/dates';
import type { Config } from './config';
import type { Day } from '../charts/geometry';

const DAY = 86_400_000;
/** Today in the workspace zone, as a day string. */
export const today = async () => localDay(new Date(), await workspaceZone());
const back = (day: string, n: number) => new Date(Date.parse(`${day}T00:00:00Z`) - n * DAY).toISOString().slice(0, 10);

/** `auto` sends no bounds so the backend picks the entity's own dates; the rest count back from today. */
export function bounds(range: Config['range'], day: string): { from?: string; to?: string } {
  if (range === 'auto') return {};
  // shortcut: "all" looks back two years, when scopes older than that matter make it a setting.
  return { from: back(day, range === 'all' ? 730 : Number(range) - 1), to: day };
}

/** Days before the scope existed say nothing; a long range would otherwise open with a flat empty stretch. */
export const trim = (days: Day[]) => days.slice(Math.max(0, days.findIndex((d) => d.scope > 0)));
