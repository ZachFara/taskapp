import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, setApi } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import type { Layout, SavedView, ViewDef } from '../../lib/bindings';
import GanttView from './GanttView.svelte';
import { insightsApi } from '../insights/insightsApi';
import ProjectBoardView from './ProjectBoardView.svelte';
import ProjectCalendarView from './ProjectCalendarView.svelte';
import ProjectListView from './ProjectListView.svelte';
import ProjectTableView from './ProjectTableView.svelte';

vi.setConfig({ testTimeout: 30000 });
const TODAY = '2026-12-02';
const def = (layout: Layout, over: Partial<ViewDef> = {}): ViewDef => ({
  filter: null, sorts: [], group_by: null, layout, columns: [], include_archived: false, density: 'comfortable', show_empty_groups: false, show_sub_issues: 'nest', card_fields: [], show_weekends: true, ...over,
});
const dated = fixtures.projects.map((p, i) => ({ ...p, status_id: i === 0 ? 1 : 2, priority: i + 1, start_date: `2026-12-0${i + 1}`, target_date: `2026-12-1${i + 1}` }));

let live: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (live) unmount(live);
  live = undefined;
  document.body.innerHTML = '';
});

async function show(Comp: typeof ProjectListView | typeof GanttView, layout: Layout, over: Partial<ViewDef> = {}, ready = '[data-slug]') {
  const view: SavedView = { id: 1, name: 'P', entity: 'project', def: def(layout, over), position: 0, icon: null, color: null, template_id: null, folder_id: null };
  setApi(createMockApi({ ...fixtures, projects: structuredClone(dated), views: [view] }));
  live = mount(Comp as typeof ProjectListView, { target: document.body, props: { view, ...(layout === 'Calendar' || layout === 'Gantt' ? { today: TODAY } : {}) } });
  await vi.waitFor(() => expect(document.querySelector(ready)).not.toBeNull());
  flushSync();
  return view;
}
const slugs = () => [...document.querySelectorAll('[data-slug]')].map((e) => (e as HTMLElement).dataset.slug);

describe('project list', () => {
  it('changes status from the row without opening the project', async () => {
    await show(ProjectListView, 'List');
    const originalPath = location.pathname;
    document.querySelector<HTMLButtonElement>('button[aria-label="Change status of Website relaunch"]')!.click();
    await vi.waitFor(() => expect(document.querySelector('[role="listbox"][aria-label="Project status"]')).not.toBeNull());
    [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((o) => o.textContent?.trim() === 'Paused')!.click();
    await vi.waitFor(async () => expect((await api.projects.list()).find((p) => p.slug === 'website')?.status_id).toBe(3));
    expect(location.pathname).toBe(originalPath);
  });
  it('has a row per project, grouped in bands by the view', async () => {
    await show(ProjectListView, 'List', { group_by: { Fixed: 'Status' } });
    expect(slugs()).toEqual(['website', 'mobile', 'office']);
    expect([...document.querySelectorAll('.band .name')].map((e) => e.textContent)).toEqual(['Planned', 'In progress']);
  });
  it('a click on a row opens the project', async () => {
    const go = vi.fn();
    addEventListener('popstate', go);
    await show(ProjectListView, 'List');
    document.querySelector<HTMLElement>('[data-slug="mobile"]')!.click();
    expect(location.pathname).toBe('/project/mobile');
    removeEventListener('popstate', go);
  });
  it('says so when the view has no projects', async () => {
    await show(ProjectListView, 'List', { filter: { Cond: { field: { Fixed: 'Title' }, op: 'Contains', value: 'zzz' } } }, '[role="img"], h2, p');
    expect(document.body.textContent).toContain('No projects in this view');
  });
});

describe('progress chart field', () => {
  const days = [0, 1, 2].map((i) => ({ date: `2026-12-0${i + 1}`, scope: 4, started: 0, completed: i }));
  afterEach(() => insightsApi.setSource(null));
  it('is off by default and draws percent plus scope and completed lines when shown', async () => {
    insightsApi.setSource({ progress: async () => ({ days, measure: 'count', from: '', to: '' }), velocity: async () => [] });
    await show(ProjectListView, 'List');
    expect(document.querySelector('.pspark')).toBeNull();
    unmount(live!);
    document.body.innerHTML = '';
    await show(ProjectListView, 'List', { columns: [{ Fixed: 'ProgressChart' }] });
    await vi.waitFor(() => expect(document.querySelector('.pspark path.completed')).not.toBeNull());
    expect(document.querySelector('.pspark path.scope')).not.toBeNull();
    expect(document.querySelector('.pspark .pct')!.textContent).toMatch(/^\d+%$/);
  });
});

describe('project table', () => {
  it('changes a displayed status and lets undo restore it', async () => {
    await show(ProjectTableView, 'Table', { columns: [{ Fixed: 'Status' }] });
    document.querySelector<HTMLButtonElement>('button[aria-label="Change status of Website relaunch"]')!.click();
    await vi.waitFor(() => expect(document.querySelector('[role="listbox"][aria-label="Project status"]')).not.toBeNull());
    [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((o) => o.textContent?.trim() === 'Paused')!.click();
    await vi.waitFor(async () => expect((await api.projects.list()).find((p) => p.slug === 'website')?.status_id).toBe(3));
    await api.history.undo();
    expect((await api.projects.list()).find((p) => p.slug === 'website')?.status_id).toBe(1);
  });

  it('shows the columns Display chose', async () => {
    await show(ProjectTableView, 'Table', { columns: [{ Fixed: 'Priority' }, { Fixed: 'TargetDate' }] });
    expect([...document.querySelectorAll('thead th')].map((h) => h.textContent)).toEqual(['Project', 'Priority', 'Target']);
  });
});

describe('project board', () => {
  it('has a column per project status, empty ones too', async () => {
    await show(ProjectBoardView, 'Board');
    const cols = [...document.querySelectorAll('[data-column]')].map((e) => (e as HTMLElement).dataset.column);
    expect(cols).toEqual(['Planned', 'In progress', 'Paused', 'Done', 'Cancelled']);
  });
  it('dropping a card on another column sets its status', async () => {
    await show(ProjectBoardView, 'Board');
    const card = document.querySelector<HTMLElement>('[data-slug="website"]')!;
    const data = new Map<string, string>();
    const dt = { setData: (k: string, v: string) => data.set(k, v), getData: (k: string) => data.get(k) ?? '', effectAllowed: '' };
    const ev = (type: string) => Object.assign(new Event(type, { bubbles: true, cancelable: true }), { dataTransfer: dt });
    card.dispatchEvent(ev('dragstart'));
    document.querySelector('[data-column="Paused"]')!.dispatchEvent(ev('drop'));
    await vi.waitFor(async () => expect((await api.projects.list()).find((p) => p.slug === 'website')!.status_id).toBe(3));
    await vi.waitFor(async () => expect((await api.history.undoStatus()).undo).toBeTruthy());
    await api.history.undo();
    expect((await api.projects.list()).find((p) => p.slug === 'website')!.status_id).toBe(1);
  });
  it('groups by priority when the view says so', async () => {
    await show(ProjectBoardView, 'Board', { group_by: { Fixed: 'Priority' } });
    expect([...document.querySelectorAll('[data-column]')].map((e) => (e as HTMLElement).dataset.column)[0]).toBe('Urgent');
  });
});

describe('project timeline', () => {
  it('draws a bar from start to target per project and a today line', async () => {
    await show(GanttView, 'Gantt', {}, '[data-row]');
    expect(document.querySelectorAll('[data-row^="project-"]').length).toBe(3);
    expect(document.querySelector('[data-testid="today"]')).not.toBeNull();
    expect(document.querySelector('.ms-mark[aria-label^="Milestone Soft launch"]')).not.toBeNull();
  });
  it('dragging a bar end changes the target date', async () => {
    await show(GanttView, 'Gantt', {}, '[data-row]');
    const handle = document.querySelector<HTMLElement>('[data-row="project-1"] [data-handle="end"]')!;
    const fire = (t: EventTarget, type: string, x: number) => t.dispatchEvent(new MouseEvent(type, { clientX: x, bubbles: true }));
    fire(handle, 'pointerdown', 0);
    fire(window, 'pointermove', 28); // week zoom: 14px a day
    fire(window, 'pointerup', 28);
    await vi.waitFor(async () => expect((await api.projects.list())[0].target_date).toBe('2026-12-13'));
  });
});

describe('project calendar', () => {
  it('puts a chip on each start and target day and opens the project on click', async () => {
    await show(ProjectCalendarView, 'Calendar', {}, '.chip');
    const chip = (day: string) => document.querySelector(`[data-day="${day}"] .chip`);
    expect(chip('2026-12-01')?.textContent).toContain('Website relaunch');
    expect(chip('2026-12-11')?.getAttribute('data-kind')).toBe('target');
    (chip('2026-12-02') as HTMLElement).click();
    expect(location.pathname).toBe('/project/mobile');
  });
});
