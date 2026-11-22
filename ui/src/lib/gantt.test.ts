import { describe, expect, it } from 'vitest';
import type { Bar, Issue, Project } from './bindings';
import { buildRows, rollup, unscheduled } from './gantt';
import { fixtures } from './api.mock';

const issue = (id: number, o: Partial<Issue> = {}): Issue => ({ ...fixtures.issues[0], id, key: `ABC-${id}`, title: `t${id}`, parent_id: null, project_id: 1, start_date: null, due: null, estimate_minutes: null, ...o });
const bar = (issue_id: number, start: string, end: string): Bar => ({ issue_id, start, end, from_estimate: false });
const base = { mode: 'issues' as const, projects: fixtures.projects as Project[], milestones: [], critical: [] };

describe('gantt rows', () => {
  it('rollup_spans_children', () => {
    expect(rollup([{ start: '2026-12-10', end: '2026-12-12' }, { start: '2026-12-02', end: '2026-12-05' }])).toEqual({ start: '2026-12-02', end: '2026-12-12' });
    const issues = [issue(3), issue(4, { parent_id: 3 }), issue(5, { parent_id: 3 })];
    const rows = buildRows({ ...base, issues, bars: [bar(4, '2026-12-10', '2026-12-12'), bar(5, '2026-12-15', '2026-12-16')] });
    expect(rows.find((r) => r.issueId === 3)).toMatchObject({ kind: 'rollup', start: '2026-12-10', end: '2026-12-16' });
  });

  it('parent_without_dated_children_has_no_rollup', () => {
    const issues = [issue(7), issue(8, { parent_id: 7 })];
    const rows = buildRows({ ...base, issues, bars: [] });
    expect(rows).toEqual([]);
  });

  it('groups_by_project_only_when_several', () => {
    const issues = [issue(1), issue(2, { project_id: 2 })];
    const bars = [bar(1, '2026-12-01', '2026-12-02'), bar(2, '2026-12-05', '2026-12-06')];
    const grouped = buildRows({ ...base, issues, bars });
    expect(grouped.filter((r) => r.kind === 'group')).toMatchObject([{ label: 'Website relaunch', start: '2026-12-01' }, { label: 'Mobile app', end: '2026-12-06' }]);
    expect(buildRows({ ...base, issues: [issues[0]], bars: [bars[0]] }).some((r) => r.kind === 'group')).toBe(false);
  });

  it('unscheduled_needs_estimate_and_no_dates', () => {
    const list = unscheduled([issue(1, { estimate_minutes: 60 }), issue(2), issue(3, { estimate_minutes: 60, start_date: '2026-12-01' })]);
    expect(list.map((i) => i.id)).toEqual([1]);
  });
});
