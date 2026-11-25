import { describe, expect, it } from 'vitest';
import { fixtures } from './api.mock';
import type { ProjectRow, ViewDef } from './bindings';
import { dateChips, groupRows, monthDays, shownFields, type PCtx } from './projectViews';
import { viewDefOf } from './projectFields';

const statuses = [
  { id: 1, name: 'Planned', category: 'Planned', position: 0, color: 'gray', color_hex: '', icon: null },
  { id: 2, name: 'In progress', category: 'Started', position: 1, color: 'amber', color_hex: '', icon: null },
  { id: 3, name: 'Done', category: 'Completed', position: 2, color: 'indigo', color_hex: '', icon: null },
] as const;
const area = { key: 'area', name: 'Area', kind: 'Select', options: [{ value: 'home', color: 'green' }], project_id: null, position: 0, applies_to: 'Projects' } as const;
const ctx: PCtx = { statuses: [...statuses], labels: [], defs: [area] };
const row = (id: number, name: string, over: Record<string, unknown> = {}): ProjectRow => ({
  project: { ...fixtures.projects[0], id, name, slug: name.toLowerCase(), ...over }, issue_total: 2, issue_done: 1,
});
const rows = [row(1, 'Alpha', { status_id: 2, priority: 3, properties: { area: 'home' } }), row(2, 'Beta', { status_id: 1 }), row(3, 'Gamma', { status_id: 2, priority: 1 })];
const names = (g: ReturnType<typeof groupRows>) => g.map((x) => x.rows.map((r) => r.project.name));

describe('groupRows', () => {
  it('is one nameless group without a group field', () => {
    expect(names(groupRows(rows, null, ctx))).toEqual([['Alpha', 'Beta', 'Gamma']]);
  });
  it('groups by status in status order and drops the empty ones, unless asked to keep them', () => {
    const g = groupRows(rows, { Fixed: 'Status' }, ctx);
    expect(g.map((x) => x.name)).toEqual(['Planned', 'In progress']);
    expect(groupRows(rows, { Fixed: 'Status' }, ctx, { empty: true }).map((x) => x.name)).toEqual(['Planned', 'In progress', 'Done']);
  });
  it('groups by priority, urgent first and none last', () => {
    expect(groupRows(rows, { Fixed: 'Priority' }, ctx).map((x) => x.name)).toEqual(['Urgent', 'Medium', 'No priority']);
  });
  it('groups by a select property with a None group', () => {
    const g = groupRows(rows, { Property: 'area' }, ctx);
    expect(g.map((x) => x.name)).toEqual(['home', 'None']);
  });
  it('a group says what dropping a project on it sets', () => {
    const [planned, started] = groupRows(rows, { Fixed: 'Status' }, ctx);
    expect(started.patch(rows[1].project).status_id).toBe(2);
    expect(planned.patch(rows[0].project).status_id).toBe(1);
    const prio = groupRows(rows, { Fixed: 'Priority' }, ctx)[0];
    expect(prio.patch(rows[1].project).priority).toBe(1);
    expect(groupRows(rows, { Property: 'area' }, ctx)[0].patch(rows[1].project).properties.area).toBe('home');
  });
  it('a field that cannot group falls back to status only when asked', () => {
    expect(groupRows(rows, { Fixed: 'TargetDate' }, ctx).length).toBe(1);
    expect(groupRows(rows, { Fixed: 'TargetDate' }, ctx, { fallback: true })[0].name).toBe('Planned');
  });
});

describe('shownFields', () => {
  const def = (over: Partial<ViewDef>): ViewDef => ({ ...viewDefOf('name', 'none', { status: null, priority: null, label: null }), ...over });
  it('uses the layout default until the view lists its own', () => {
    expect(shownFields(def({ layout: 'Table' })).map((f) => f.Fixed)).toContain('IssueCount');
    expect(shownFields(def({ layout: 'Table', columns: [{ Fixed: 'Priority' }] }))).toEqual([{ Fixed: 'Priority' }]);
  });
  it('a board draws its card fields when it has them', () => {
    expect(shownFields(def({ layout: 'Board', columns: [{ Fixed: 'Status' }], card_fields: [{ Fixed: 'Label' }] }), true)).toEqual([{ Fixed: 'Label' }]);
  });
});

describe('the Projects page as a view', () => {
  it('keeps its filters, sort and grouping', () => {
    const d = viewDefOf('target', 'status', { status: 2, priority: 3, label: null });
    expect(d.filter).toEqual({ And: [{ Cond: { field: { Fixed: 'Status' }, op: 'Eq', value: 2 } }, { Cond: { field: { Fixed: 'Priority' }, op: 'Eq', value: 3 } }] });
    expect(d.sorts).toEqual([{ field: { Fixed: 'TargetDate' }, descending: false }]);
    expect(d.group_by).toEqual({ Fixed: 'Status' });
    expect(viewDefOf('name', 'none', { status: null, priority: null, label: null }).filter).toBeNull();
  });
});

describe('calendar helpers', () => {
  it('a month grid is whole weeks, starting on the chosen day', () => {
    const sun = monthDays(2027, 0, false); // Jan 2027 starts on a Friday
    expect(sun[0].day).toBe('2026-12-27');
    expect(sun.length % 7).toBe(0);
    expect(monthDays(2027, 0, true)[0].day).toBe('2026-12-28');
    expect(sun.filter((d) => d.inMonth).length).toBe(31);
  });
  it('a project has a chip on its start and on its target day', () => {
    const chips = dateChips([row(1, 'A', { start_date: '2027-01-02', target_date: '2027-01-20' }), row(2, 'B', { start_date: null, target_date: '2027-01-05' })]);
    expect(chips.map((c) => `${c.row.project.name}:${c.kind}:${c.day}`)).toEqual(['A:start:2027-01-02', 'A:target:2027-01-20', 'B:target:2027-01-05']);
  });
});
