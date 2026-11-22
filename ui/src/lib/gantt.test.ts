import { describe, expect, it } from 'vitest';
import type { Bar, Issue, Project } from './bindings';
import { buildRows, dragDates, localDay, rollup, saturdays, shiftLocalDays, tickLabel, unscheduled, zoomBy } from './gantt';
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

  it('tick_labels_follow_the_zoom', () => {
    expect(tickLabel('2026-12-07', 'Day')).toBe('Mon 7');
    expect(tickLabel('2026-12-07', 'Week')).toBe('Dec 7');
    expect(tickLabel('2026-12-07', 'Quarter')).toBe('Dec 2026');
  });
});
