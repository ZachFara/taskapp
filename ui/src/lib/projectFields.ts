// How the Projects page filters, sorts and groups, and how a project status draws: pure, so the page stays thin.
import type { Filter, FixedField, Project, ProjectStatus, ProjectStatusCategory, StatusCategory, ViewDef } from './bindings';
import { priorityName, priorityOrder } from './priority';

/** A project status reuses the issue stone glyphs; the five categories land on five different ones. */
export const glyphCategory: Record<ProjectStatusCategory, StatusCategory> = {
  Planned: 'Backlog', Paused: 'Unstarted', Started: 'Started', Completed: 'Completed', Cancelled: 'Canceled',
};

export type Sort = 'name' | 'status' | 'priority' | 'target';
export type Group = 'none' | 'status' | 'priority';
export type Filters = { status: number | null; priority: number | null; label: number | null };
export const noFilters: Filters = { status: null, priority: null, label: null };

type Row = Project;

/** Rows after the filters, in the chosen order, split into groups (one nameless group when not grouping). */
export function arrange(rows: Row[], statuses: ProjectStatus[], opts: { sort: Sort; group: Group; filters: Filters }) {
  const { sort, group, filters } = opts;
  const pos = (p: Row) => statuses.find((s) => s.id === p.status_id)?.position ?? 99;
  const kept = rows.filter((p) =>
    (filters.status === null || p.status_id === filters.status) &&
    (filters.priority === null || p.priority === filters.priority) &&
    (filters.label === null || p.label_ids.includes(filters.label)));
  const by: Record<Sort, (a: Row, b: Row) => number> = {
    name: (a, b) => a.name.localeCompare(b.name),
    status: (a, b) => pos(a) - pos(b),
    priority: (a, b) => priorityOrder(a.priority) - priorityOrder(b.priority),
    // No target date last.
    target: (a, b) => (a.target_date ?? '9999').localeCompare(b.target_date ?? '9999'),
  };
  const sorted = [...kept].sort((a, b) => by[sort](a, b) || a.name.localeCompare(b.name));
  if (group === 'none') return [{ key: 'all', name: '', status: null as ProjectStatus | null, rows: sorted }];
  if (group === 'priority') {
    return [1, 2, 3, 4, 0]
      .map((n) => ({ key: `p${n}`, name: priorityName(n), status: null, rows: sorted.filter((p) => p.priority === n) }))
      .filter((g) => g.rows.length);
  }
  return statuses
    .map((s) => ({ key: `s${s.id}`, name: s.name, status: s as ProjectStatus | null, rows: sorted.filter((p) => p.status_id === s.id) }))
    .filter((g) => g.rows.length);
}

const SORT_FIELD: Record<Sort, FixedField> = { name: 'Title', status: 'Status', priority: 'Priority', target: 'TargetDate' };

/** The project view that shows what the Projects page shows now: its filters, sort and grouping, as a table. */
export function viewDefOf(sort: Sort, group: Group, filters: Filters): ViewDef {
  const cond = (f: FixedField, value: number): Filter => ({ Cond: { field: { Fixed: f }, op: 'Eq', value } });
  const parts = [
    ...(filters.status === null ? [] : [cond('Status', filters.status)]),
    ...(filters.priority === null ? [] : [cond('Priority', filters.priority)]),
    ...(filters.label === null ? [] : [cond('Label', filters.label)]),
  ];
  return {
    filter: parts.length === 0 ? null : parts.length === 1 ? parts[0] : { And: parts },
    sorts: [{ field: { Fixed: SORT_FIELD[sort] }, descending: false }],
    group_by: group === 'none' ? null : { Fixed: group === 'status' ? 'Status' : 'Priority' },
    layout: 'Table', columns: [], include_archived: false,
    density: 'comfortable', show_empty_groups: false, show_sub_issues: 'nest', card_fields: [], show_weekends: true, wrap: 'off',
  };
}
