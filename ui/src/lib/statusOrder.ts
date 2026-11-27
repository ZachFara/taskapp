import type { FlowKind, Status, StatusCategory } from './bindings';

export const STATUS_CATEGORIES: StatusCategory[] = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'];

type Placed = Pick<Status, 'category' | 'position'>;

/** The order statuses are listed in everywhere: by category, then the owner's position inside it. */
export const byStatusOrder = (a: Placed, b: Placed) =>
  STATUS_CATEGORIES.indexOf(a.category) - STATUS_CATEGORIES.indexOf(b.category) || a.position - b.position;

/** A single sortable number with the same order as `byStatusOrder` (for group headers). */
export const statusRank = (s: Placed) => STATUS_CATEGORIES.indexOf(s.category) * 1e6 + s.position;

/** What a new status gets, like the backend: started work is active, the rest idle. */
export const flowDefaults = (category: StatusCategory): { definition: string; flow_kind: FlowKind; nudge_days: number | null } => ({
  definition: '',
  flow_kind: category === 'Started' ? 'Active' : 'Idle',
  nudge_days: null,
});
