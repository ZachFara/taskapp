import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../../lib/api';
import { api } from '../../lib/api';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import type { Issue, SavedView, ViewDef } from '../../lib/bindings';
import GanttView from './GanttView.svelte';

const def: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'Gantt', columns: [], include_archived: false };
const issue = (id: number, o: Partial<Issue> = {}): Issue => ({ ...fixtures.issues[0], id, key: `ABC-${id}`, title: `t${id}`, parent_id: null, project_id: 1, start_date: null, due: null, estimate_minutes: null, ...o });
const dated = (id: number, start: string, end: string, o: Partial<Issue> = {}) => issue(id, { start_date: start, due: { Date: end }, ...o });

let view: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (view) unmount(view);
  view = undefined;
  document.body.innerHTML = '';
});

async function show(issues: Issue[], props: { mode?: 'issues' | 'projects'; view?: SavedView; today?: string } = {}, seed: Partial<MockSeed> = {}) {
  setApi(createMockApi({ ...fixtures, issues, views: [{ id: 1, name: 'All', def, position: 0 }], ...seed }));
  if (seed.issues) throw new Error('pass issues as the first argument');
  view = mount(GanttView, { target: document.body, props: { view: { id: 1, name: 'All', def, position: 0 }, today: '2026-12-02', ...props } });
  await vi.waitFor(() => expect(document.querySelector('.timeline')).not.toBeNull());
  await vi.waitFor(() => expect(api.views.list).toBeDefined());
  await new Promise((r) => setTimeout(r, 20));
  flushSync();
}
const bar = (id: string) => document.querySelector<HTMLElement>(`[data-row="${id}"]`)!;
const fire = (target: EventTarget, type: string, x: number) => target.dispatchEvent(new MouseEvent(type, { clientX: x, bubbles: true }));
const dragBy = (el: Element, px: number) => {
  fire(el, 'pointerdown', 0);
  fire(window, 'pointermove', px);
  fire(window, 'pointerup', px);
};
// Week zoom is 14px a day.
const chain = () => [dated(1, '2026-12-01', '2026-12-03'), dated(2, '2026-12-05', '2026-12-07')];
const block = () => api.relations.add({ from: 1, to: 2, kind: 'Blocks' });

describe('GanttView', () => {
  it('drag_confirms_and_applies_shift', async () => {
    await show(chain());
    await block();
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1'), 28);
    await vi.waitFor(() => expect(document.querySelector('.toast')?.textContent).toContain('Also moves ABC-2'));
    expect(update).not.toHaveBeenCalled();
    (document.querySelectorAll('.toast button')[0] as HTMLElement).click();
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(2));
    expect((await api.issues.get(1)).start_date).toBe('2026-12-03');
  });

  it('declining_the_shift_applies_nothing', async () => {
    await show(chain());
    await block();
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1'), 28);
    await vi.waitFor(() => expect(document.querySelector('.toast')).not.toBeNull());
    (document.querySelectorAll('.toast button')[1] as HTMLElement).click();
    flushSync();
    expect(document.querySelector('.toast')).toBeNull();
    expect(update).not.toHaveBeenCalled();
  });

  it('critical_path_class_applied', async () => {
    await show([...chain(), dated(3, '2026-12-01', '2026-12-02')]);
    await block();
    // the view loaded before the relation existed; remount to reload
    unmount(view!);
    view = mount(GanttView, { target: document.body, props: { view: { id: 1, name: 'All', def, position: 0 }, today: '2026-12-02' } });
    await vi.waitFor(() => expect(bar('issue-1')?.classList.contains('critical')).toBe(true));
    expect(bar('issue-2').classList.contains('critical')).toBe(true);
    expect(bar('issue-3').classList.contains('critical')).toBe(false);
  });

  it('drop_unscheduled_sets_start', async () => {
    await show([dated(1, '2026-12-01', '2026-12-03'), issue(9, { estimate_minutes: 480 })]);
    const item = document.querySelector('[data-issue="9"]')!;
    item.dispatchEvent(new Event('dragstart', { bubbles: true }));
    // range starts 3 days before the first bar: 2026-11-28, so day 5 is 2026-12-03
    document.querySelector('.timeline')!.dispatchEvent(new MouseEvent('drop', { clientX: 70, bubbles: true, cancelable: true }));
    await vi.waitFor(async () => expect((await api.issues.get(9)).start_date).toBe('2026-12-03'));
  });

  it('zoom_levels_switch', async () => {
    await show(chain());
    const width = () => parseFloat(document.querySelector<HTMLElement>('.canvas')!.style.width);
    const widths: number[] = [];
    for (const z of ['Day', 'Week', 'Month', 'Quarter']) {
      ([...document.querySelectorAll('header [role=tab]')].find((b) => b.textContent?.trim() === z) as HTMLElement).focus();
      flushSync();
      expect(document.querySelector('.gantt')!.getAttribute('data-zoom')).toBe(z);
      widths.push(width());
    }
    expect(widths).toEqual([...widths].sort((a, b) => b - a));
    expect(new Set(widths).size).toBe(4);
  });

  it('multi_project_timeline_groups_by_project', async () => {
    await show([dated(1, '2026-12-01', '2026-12-03'), dated(2, '2026-12-05', '2026-12-07', { project_id: 2 })]);
    const labels = [...document.querySelectorAll('.label.group .text')].map((l) => l.textContent);
    expect(labels).toEqual([fixtures.projects[0].name, fixtures.projects[1].name]);
    expect(bar('group-1')).not.toBeNull();
    expect(bar('group-2')).not.toBeNull();
  });

  it('projects_mode_drag_updates_target_date', async () => {
    const projects = fixtures.projects.map((p) => (p.id === 1 ? { ...p, start_date: '2026-12-01', target_date: '2026-12-31' } : p));
    await show([], { mode: 'projects' }, { projects });
    dragBy(bar('project-1'), 28);
    await vi.waitFor(async () => expect((await api.projects.list())[0].target_date).toBe('2027-01-02'));
    expect((await api.projects.list())[0].start_date).toBe('2026-12-03');
  });

  it('empty_view_shows_empty_state_and_unscheduled_lane', async () => {
    await show([issue(9, { estimate_minutes: 480 })], {}, { cycles: [], milestones: [] });
    expect(document.querySelector('.empty')?.textContent).toContain('Nothing scheduled');
    expect(document.querySelector('.lane')?.textContent).toContain('ABC-9');
  });

  it('parent_without_dated_children_has_no_rollup_bar', async () => {
    await show([issue(7), issue(8, { parent_id: 7 }), dated(1, '2026-12-01', '2026-12-03')]);
    expect(bar('issue-7')).toBeNull();
    expect(bar('issue-1')).not.toBeNull();
  });

  it('drag_keeps_datetime_due_time_and_zone', async () => {
    const due = { DateTime: { utc: '2026-12-03T15:00:00Z', tz: 'US/Eastern' } };
    await show([issue(1, { start_date: '2026-12-01', due })]);
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1'), 28);
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    expect(update).toHaveBeenCalledWith(1, { start_date: '2026-12-03', due: { DateTime: { utc: '2026-12-05T15:00:00Z', tz: 'US/Eastern' } } });
  });

  it('drag_across_dst_keeps_local_wall_clock', async () => {
    const due = { DateTime: { utc: '2026-03-06T14:00:00Z', tz: 'US/Eastern' } };
    await show([issue(1, { start_date: '2026-03-06', due })]);
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1'), 42);
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    expect(update.mock.calls[0][1]).toEqual({ start_date: '2026-03-09', due: { DateTime: { utc: '2026-03-09T13:00:00Z', tz: 'US/Eastern' } } });
  });

  it('drag_estimate_only_bar_does_not_invent_due', async () => {
    await show([issue(1, { start_date: '2026-12-01', estimate_minutes: 480 })]);
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1'), 28);
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    expect(update.mock.calls[0][1]).toEqual({ start_date: '2026-12-03' });
  });

  it('failed_apply_reloads_and_shows_error', async () => {
    await show(chain());
    await block();
    const real = api.issues.update.bind(api.issues);
    let n = 0;
    vi.spyOn(api.issues, 'update').mockImplementation((...a) => (++n === 2 ? Promise.reject({ message: 'boom' }) : real(...a)));
    const rejections: unknown[] = [];
    const on = (e: PromiseRejectionEvent) => { rejections.push(e.reason); e.preventDefault(); };
    window.addEventListener('unhandledrejection', on);
    dragBy(bar('issue-1'), 28);
    await vi.waitFor(() => expect(document.querySelector('.toast')).not.toBeNull());
    (document.querySelectorAll('.toast button')[0] as HTMLElement).click();
    await vi.waitFor(() => expect(document.body.textContent).toContain('boom'));
    await new Promise((r) => setTimeout(r, 20));
    window.removeEventListener('unhandledrejection', on);
    expect(rejections).toEqual([]);
  });

  it('rollup_rows_are_not_draggable', async () => {
    await show([issue(7), dated(8, '2026-12-01', '2026-12-03', { parent_id: 7 })]);
    const update = vi.spyOn(api.issues, 'update');
    const before = bar('issue-7').style.left;
    dragBy(bar('issue-7'), 28);
    await new Promise((r) => setTimeout(r, 20));
    flushSync();
    expect(update).not.toHaveBeenCalled();
    expect(bar('issue-7').style.left).toBe(before);
  });

  it('resize_end_edge_changes_only_due', async () => {
    await show([dated(1, '2026-12-01', '2026-12-03')]);
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1').querySelector('[data-handle="end"]')!, 28);
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    expect(update).toHaveBeenCalledWith(1, { start_date: '2026-12-01', due: { Date: '2026-12-05' } });
  });

  it('resize_start_edge_changes_only_start', async () => {
    await show([dated(1, '2026-12-01', '2026-12-03')]);
    const update = vi.spyOn(api.issues, 'update');
    dragBy(bar('issue-1').querySelector('[data-handle="start"]')!, -14);
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    expect(update).toHaveBeenCalledWith(1, { start_date: '2026-11-30', due: { Date: '2026-12-03' } });
  });

  it('zoom_commands_step_the_scale', async () => {
    await show(chain());
    const zoom = () => document.querySelector('.gantt')!.getAttribute('data-zoom');
    window.dispatchEvent(new CustomEvent('command:timeline.zoom-in'));
    flushSync();
    expect(zoom()).toBe('Day');
    window.dispatchEvent(new CustomEvent('command:timeline.zoom-out'));
    window.dispatchEvent(new CustomEvent('command:timeline.zoom-out'));
    flushSync();
    expect(zoom()).toBe('Month');
  });

  it('today_marker_sits_on_its_day', async () => {
    await show(chain());
    // the axis starts 2026-11-28 (three days before the first bar), 14px a day: 4 days to 12-02
    expect(document.querySelector<HTMLElement>('[data-testid="today"]')!.style.left).toBe('56px');
  });

  it('bars_carry_the_status_category', async () => {
    await show([dated(1, '2026-12-01', '2026-12-03')]);
    const want = (await api.statuses.list()).find((s) => s.id === fixtures.issues[0].status_id)!.category;
    expect(bar('issue-1').getAttribute('data-category')).toBe(want);
  });
});
