import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../../lib/api';
import { api } from '../../lib/api';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import type { Issue, SavedView, ViewDef } from '../../lib/bindings';
import GanttView from './GanttView.svelte';

const def: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'Gantt', columns: [], include_archived: false };
const issue = (id: number, o: Partial<Issue> = {}): Issue => ({ ...fixtures.issues[0], id, key: `ZF-${id}`, title: `t${id}`, parent_id: null, project_id: 1, start_date: null, due: null, estimate_minutes: null, ...o });
const dated = (id: number, start: string, end: string, o: Partial<Issue> = {}) => issue(id, { start_date: start, due: { Date: end }, ...o });

let view: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (view) unmount(view);
  view = undefined;
  document.body.innerHTML = '';
});

async function show(issues: Issue[], props: { mode?: 'issues' | 'projects'; view?: SavedView } = {}, seed: Partial<MockSeed> = {}) {
  setApi(createMockApi({ ...fixtures, issues, views: [{ id: 1, name: 'All', def, position: 0 }], ...seed }));
  if (seed.issues) throw new Error('pass issues as the first argument');
  view = mount(GanttView, { target: document.body, props: { view: { id: 1, name: 'All', def, position: 0 }, ...props } });
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
    await vi.waitFor(() => expect(document.querySelector('.toast')?.textContent).toContain('Also moves ZF-2'));
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
    view = mount(GanttView, { target: document.body, props: { view: { id: 1, name: 'All', def, position: 0 } } });
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
      [...document.querySelectorAll('header button')].find((b) => b.textContent === z)!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      flushSync();
      expect(document.querySelector('.gantt')!.getAttribute('data-zoom')).toBe(z);
      widths.push(width());
    }
    expect(widths).toEqual([...widths].sort((a, b) => b - a));
    expect(new Set(widths).size).toBe(4);
  });

  it('multi_project_timeline_groups_by_project', async () => {
    await show([dated(1, '2026-12-01', '2026-12-03'), dated(2, '2026-12-05', '2026-12-07', { project_id: 2 })]);
    const labels = [...document.querySelectorAll('.label.group')].map((l) => l.textContent);
    expect(labels).toEqual(['Home', 'Work']);
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
    await show([issue(9, { estimate_minutes: 480 })]);
    expect(document.querySelector('.empty')?.textContent).toContain('Nothing scheduled');
    expect(document.querySelector('.lane')?.textContent).toContain('ZF-9');
  });

  it('parent_without_dated_children_has_no_rollup_bar', async () => {
    await show([issue(7), issue(8, { parent_id: 7 }), dated(1, '2026-12-01', '2026-12-03')]);
    expect(bar('issue-7')).toBeNull();
    expect(bar('issue-1')).not.toBeNull();
  });
});
