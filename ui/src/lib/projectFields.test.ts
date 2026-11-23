import { describe, expect, it } from 'vitest';
import { fixtures } from './api.mock';
import { arrange, noFilters } from './projectFields';
import type { Project } from './bindings';

const statuses = [
  { id: 1, name: 'Planned', category: 'Planned', position: 0, color: 'gray', color_hex: '', icon: null },
  { id: 2, name: 'In progress', category: 'Started', position: 1, color: 'amber', color_hex: '', icon: null },
] as const;
const mk = (id: number, name: string, over: Partial<Project>): Project => ({ ...fixtures.projects[0], id, name, slug: name.toLowerCase(), ...over });
const rows = [
  mk(1, 'Beta', { status_id: 2, priority: 3, target_date: '2027-02-01', label_ids: [1] }),
  mk(2, 'Alpha', { status_id: 1, priority: 1, target_date: null }),
  mk(3, 'Gamma', { status_id: 2, priority: 0, target_date: '2027-01-01' }),
];
const names = (g: ReturnType<typeof arrange>) => g.map((x) => x.rows.map((p) => p.name));

describe('arrange', () => {
  it('sorts by name, and a missing target date goes last', () => {
    expect(names(arrange(rows, [...statuses], { sort: 'name', group: 'none', filters: noFilters }))).toEqual([['Alpha', 'Beta', 'Gamma']]);
    expect(names(arrange(rows, [...statuses], { sort: 'target', group: 'none', filters: noFilters }))).toEqual([['Gamma', 'Beta', 'Alpha']]);
  });
  it('sorts by priority with urgent first and none last', () => {
    expect(names(arrange(rows, [...statuses], { sort: 'priority', group: 'none', filters: noFilters }))).toEqual([['Alpha', 'Beta', 'Gamma']]);
  });
  it('groups by status in the status order and drops empty groups', () => {
    const g = arrange(rows, [...statuses], { sort: 'name', group: 'status', filters: noFilters });
    expect(g.map((x) => x.name)).toEqual(['Planned', 'In progress']);
    expect(names(g)).toEqual([['Alpha'], ['Beta', 'Gamma']]);
  });
  it('groups by priority with none last', () => {
    expect(arrange(rows, [...statuses], { sort: 'name', group: 'priority', filters: noFilters }).map((x) => x.name)).toEqual(['Urgent', 'Medium', 'No priority']);
  });
  it('filters by status, priority and label together', () => {
    expect(names(arrange(rows, [...statuses], { sort: 'name', group: 'none', filters: { status: 2, priority: null, label: null } }))).toEqual([['Beta', 'Gamma']]);
    expect(names(arrange(rows, [...statuses], { sort: 'name', group: 'none', filters: { status: 2, priority: 3, label: 1 } }))).toEqual([['Beta']]);
  });
});
