import { describe, expect, it } from 'vitest';
import type { Bar, Issue, Milestone, Project } from './bindings';
import { axis, buildRows, dragDates, localDay, range, rollup, saturdays, shiftLocalDays, unscheduled, zoomBy } from './gantt';
import { fixtures, projectStatusSeed } from './api.mock';

const issue = (id: number, o: Partial<Issue> = {}): Issue => ({ ...fixtures.issues[0], id, key: `ABC-${id}`, title: `t${id}`, parent_id: null, project_id: 1, start_date: null, due: null, estimate_minutes: null, ...o });
const bar = (issue_id: number, start: string, end: string): Bar => ({ issue_id, start, end, from_estimate: false });
const base = { mode: 'issues' as const, projects: fixtures.projects as Project[], milestones: [], critical: [], today: '2026-12-02' };

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

  it('grouped_band_carries_its_projects_milestones_and_keeps_them_in_range', () => {
    const issues = [issue(1), issue(2, { project_id: 2 })];
    const bars = [bar(1, '2026-12-01', '2026-12-02'), bar(2, '2026-12-05', '2026-12-06')];
    const milestones: Milestone[] = [{ id: 9, project_id: 1, name: 'Launch', target_date: '2026-12-20' }];
    const grouped = buildRows({ ...base, issues, bars, milestones });
    expect(grouped.find((r) => r.id === 'group-1')?.milestones?.map((m) => m.name)).toEqual(['Launch']);
    expect(grouped.filter((r) => r.id.startsWith('milestone-'))).toEqual([]);
    expect(range(grouped, base.today).end >= '2026-12-20').toBe(true);
  });

  it('an_ungrouped_project_gets_a_band_only_for_its_milestones', () => {
    const milestones: Milestone[] = [{ id: 9, project_id: 1, name: 'Launch', target_date: '2026-12-20' }];
    const plain = buildRows({ ...base, issues: [issue(1)], bars: [bar(1, '2026-12-01', '2026-12-02')] });
    expect(plain.some((r) => r.kind === 'group')).toBe(false);
    const banded = buildRows({ ...base, issues: [issue(1)], bars: [bar(1, '2026-12-01', '2026-12-02')], milestones });
    expect(banded.map((r) => r.id)).toEqual(['group-1', 'issue-1']);
    expect(banded[0].milestones?.map((m) => m.name)).toEqual(['Launch']);
  });

  it('unscheduled_is_every_issue_without_a_row', () => {
    const issues = [issue(1), issue(2, { estimate_minutes: 60 }), issue(3, { start_date: '2026-12-01' })];
    const rows = buildRows({ ...base, issues, bars: [bar(3, '2026-12-01', '2026-12-02')] });
    expect(unscheduled(issues, rows).map((i) => i.id)).toEqual([1, 2]);
  });

  it('rows_carry_key_and_title_apart', () => {
    const rows = buildRows({ ...base, issues: [issue(1)], bars: [bar(1, '2026-12-01', '2026-12-02')] });
    expect(rows[0]).toMatchObject({ key: 'ABC-1', title: 't1' });
  });
});

describe('gantt project rows', () => {
  const today = '2026-12-02';
  const proj = (o: Partial<Project>) => ({ ...fixtures.projects[0], ...o }) as Project;
  const rowsOf = (projects: Project[], issues: Issue[] = [], milestones: Milestone[] = []) =>
    buildRows({ mode: 'projects', issues, bars: [], projects, milestones, critical: [], today });

  it('a_dateless_project_gets_an_undated_row', () => {
    const rows = rowsOf([proj({ start_date: null, target_date: null })]);
    expect(rows).toEqual([expect.objectContaining({ id: 'project-1', label: 'Website relaunch', projectId: 1, undated: true })]);
    expect(rows[0].start).toBeUndefined();
  });

  it('milestones_ride_on_their_project_row_and_undated_ones_are_skipped', () => {
    const milestones: Milestone[] = [
      { id: 9, project_id: 1, name: 'Launch', target_date: '2026-12-20' },
      { id: 10, project_id: 1, name: 'Someday', target_date: null },
      { id: 11, project_id: 2, name: 'Other', target_date: '2026-12-01' },
    ];
    const rows = rowsOf([proj({ start_date: null, target_date: null })], [], milestones);
    expect(rows).toHaveLength(1);
    expect(rows[0].milestones?.map((m) => m.name)).toEqual(['Launch']);
  });

  it('a_start_only_project_runs_to_today_or_its_latest_issue', () => {
    const start = proj({ start_date: '2026-12-01', target_date: null });
    expect(rowsOf([start])[0]).toMatchObject({ kind: 'bar', start: '2026-12-01', end: today, estimated: true });
    const later = [{ ...fixtures.issues[0], id: 5, project_id: 1, due: { Date: '2026-12-20' } } as Issue];
    expect(rowsOf([start], later)[0]).toMatchObject({ start: '2026-12-01', end: '2026-12-20' });
  });

  it('a_target_only_project_draws_a_week_ending_at_the_target', () => {
    expect(rowsOf([proj({ start_date: null, target_date: '2026-12-10' })])[0]).toMatchObject({ start: '2026-12-03', end: '2026-12-10', estimated: true });
  });

  it('a_project_bar_takes_its_status_colour', () => {
    const projects = [proj({ start_date: '2026-12-01', target_date: '2026-12-05', status_id: 2 })];
    const rows = buildRows({ mode: 'projects', issues: [], bars: [], projects, milestones: [], critical: [], today, projectStatuses: projectStatusSeed });
    expect(rows[0]).toMatchObject({ kind: 'bar', color: 'amber' });
  });
});

describe('gantt date math', () => {
  it('local_day_uses_the_zone_passed_in', () => {
    const t = new Date('2026-12-01T02:30:00Z');
    expect(localDay(t, 'UTC')).toBe('2026-12-01');
    expect(localDay(t, 'US/Eastern')).toBe('2026-11-30');
    expect(localDay(t, 'Pacific/Auckland')).toBe('2026-12-01');
  });

  it('shift_local_days_keeps_wall_clock_across_dst', () => {
    // US clocks spring forward on 2026-03-08: 09:00 New York is 14:00Z before and 13:00Z after.
    expect(shiftLocalDays('2026-03-06T14:00:00Z', 'US/Eastern', 3)).toBe('2026-03-09T13:00:00Z');
    expect(shiftLocalDays('2026-03-09T13:00:00Z', 'US/Eastern', -3)).toBe('2026-03-06T14:00:00Z');
    expect(shiftLocalDays('2026-03-06T14:00:00Z', 'UTC', 3)).toBe('2026-03-09T14:00:00Z');
    // Southern hemisphere zone, falls back on 2026-04-05.
    expect(shiftLocalDays('2026-04-03T00:00:00Z', 'Australia/Sydney', 3)).toBe('2026-04-06T01:00:00Z');
  });

  it('drag_dates_move_resize_and_clamp', () => {
    const s = { start: '2026-12-01', end: '2026-12-03' };
    expect(dragDates(s, 'move', 2)).toEqual({ start: '2026-12-03', end: '2026-12-05' });
    expect(dragDates(s, 'start', -2)).toEqual({ start: '2026-11-29', end: '2026-12-03' });
    expect(dragDates(s, 'end', 4)).toEqual({ start: '2026-12-01', end: '2026-12-07' });
    expect(dragDates(s, 'start', 9)).toEqual({ start: '2026-12-03', end: '2026-12-03' });
    expect(dragDates(s, 'end', -9)).toEqual({ start: '2026-12-01', end: '2026-12-01' });
  });

  it('zoom_by_steps_and_stops_at_the_ends', () => {
    expect(zoomBy('Week', 1)).toBe('Day');
    expect(zoomBy('Week', -1)).toBe('Month');
    expect(zoomBy('Day', 1)).toBe('Day');
    expect(zoomBy('Quarter', -1)).toBe('Quarter');
  });

  it('saturdays_are_offsets_from_the_start', () => {
    // 2026-11-28 is a Saturday.
    expect(saturdays('2026-11-26', 12)).toEqual([2, 9]);
  });

  it('axis_has_one_tick_per_unit_and_bands_that_tile_the_range', () => {
    const day = axis('2026-11-29', 5, 'Day');
    expect(day.ticks.map((t) => `${t.sub} ${t.label}`)).toEqual(['Sun 29', 'Mon 30', 'Tue 1', 'Wed 2', 'Thu 3']);
    expect(day.bands).toEqual([{ k: 0, len: 2, label: 'November 2026' }, { k: 2, len: 3, label: 'December 2026' }]);
    expect(axis('2026-12-01', 15, 'Week').ticks).toEqual([{ k: 6, label: 'Dec 7' }, { k: 13, label: 'Dec 14' }]);
    const q = axis('2026-12-20', 30, 'Quarter');
    expect(q.ticks).toEqual([{ k: 12, label: 'Jan' }]);
    expect(q.bands.map((b) => b.label)).toEqual(['2026', '2027']);
  });
});
