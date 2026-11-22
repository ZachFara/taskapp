import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import type { Issue, SavedView, ViewDef } from '../../lib/bindings';
import { selection } from '../../lib/selection.svelte';
import { listState } from '../../lib/listState.svelte';
import { listCommands, type Ctx } from '../../lib/commands';
import ListView from './ListView.svelte';

const groupBy: ViewDef = { filter: null, sorts: [], group_by: { Fixed: 'Status' }, layout: 'List', columns: [], include_archived: false };
const viewSeed = (def: Partial<ViewDef> = {}): SavedView[] => [
  { id: 71, name: 'Mine', def: { ...groupBy, ...def }, position: 0, icon: null, color: null },
];
const [S1, S2] = fixtures.statuses;

const issue = (n: number, over: Partial<Issue> = {}): Issue => ({
  ...fixtures.issues[0], id: n, key: `ABC-${n}`, title: `Row ${n}`, status_id: S1.id, priority: 0, project_id: null,
  cycle_id: null, label_ids: [], start_date: null, due: null, estimate_minutes: null, properties: {}, ...over,
});

let live: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  selection.clear();
  selection.focus(null);
  listState.editing = null;
  localStorage.clear();
});
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.useRealTimers();
});

async function open(seed: Partial<MockSeed>) {
  const full = { ...fixtures, views: viewSeed(), ...seed } as MockSeed;
  const mock = createMockApi(full);
  setApi(mock);
  const target = document.createElement('div');
  document.body.append(target);
  const started = performance.now();
  live = mount(ListView, { target, props: { view: full.views![0] } });
  flushSync();
  await vi.waitFor(() => expect(target.querySelector('li[data-key], .empty')).not.toBeNull(), { interval: 1 });
  return { mock, target, ms: performance.now() - started };
}
const rows = (t: HTMLElement) => [...t.querySelectorAll<HTMLElement>('li[data-key]')];
const row = (t: HTMLElement, n: number) => t.querySelector<HTMLElement>(`li[data-key="ABC-${n}"]`)!;
const bands = (t: HTMLElement) => [...t.querySelectorAll<HTMLElement>('.band')].map((b) => ({
  name: b.querySelector('.name')!.textContent, count: b.querySelector('.count')!.textContent,
}));
const ctxFor = (id: number | null): Ctx => ({ route: '/', selection: [], focusedIssueId: id, openIssueId: null, api: {} as Ctx['api'], navigate: () => {} });

describe('ListView', () => {
  it('groups_by_status_with_counts', async () => {
    const { target } = await open({ issues: [issue(1), issue(2), issue(3, { status_id: S2.id })] });
    expect(bands(target)).toEqual([{ name: S1.name, count: '2' }, { name: S2.name, count: '1' }]);
  });

  it('groups_by_priority_and_none', async () => {
    const issues = [issue(1, { priority: 1 }), issue(2, { priority: 0 })];
    const { target } = await open({ issues, views: viewSeed({ group_by: { Fixed: 'Priority' } }) });
    expect(bands(target).map((b) => b.name)).toEqual(['Urgent', 'No priority']);
    unmount(live!);
    live = null;
    document.body.innerHTML = '';
    const flat = await open({ issues, views: viewSeed({ group_by: null }) });
    expect(flat.target.querySelectorAll('.band')).toHaveLength(0);
    expect(rows(flat.target)).toHaveLength(2);
  });

  it('non_status_group_header_has_no_status_glyph_and_a_leading_icon', async () => {
    const proj = fixtures.projects[0];
    const { target } = await open({ issues: [issue(1, { project_id: proj.id })], views: viewSeed({ group_by: { Fixed: 'Project' } }) });
    const band = target.querySelector<HTMLElement>('.band')!;
    expect(band.textContent).toContain(proj.name);
    expect(band.querySelector('[data-status-glyph], .status-glyph')).toBeNull();
    expect(band.querySelectorAll('svg, .entity-icon').length).toBeGreaterThan(1);
  });

  it('publishes_its_visible_ids_as_one_column_for_j_and_k', async () => {
    const { target } = await open({ issues: [issue(1), issue(2)] });
    await vi.waitFor(() => expect(selection.columns).toEqual([rows(target).map((r) => Number(r.dataset.key!.slice(4)))]));
  });

  it('collapse_hides_rows_and_persists_per_view', async () => {
    const issues = [issue(1), issue(2, { status_id: S2.id })];
    const { target } = await open({ issues });
    target.querySelector<HTMLElement>(`[aria-label="Collapse ${S1.name}"]`)!.click();
    flushSync();
    expect(rows(target).map((r) => r.dataset.key)).toEqual(['ABC-2']);
    unmount(live!);
    live = null;
    document.body.innerHTML = '';
    const again = await open({ issues });
    expect(rows(again.target).map((r) => r.dataset.key)).toEqual(['ABC-2']);
    expect(again.target.querySelector(`[aria-label="Expand ${S1.name}"]`)).not.toBeNull();
  });

  it('click_focuses_cmd_click_toggles_shift_click_selects_range', async () => {
    const { target } = await open({ issues: [1, 2, 3, 4].map((n) => issue(n)) });
    row(target, 2).click();
    flushSync();
    expect(selection.focused).toBe(2);
    expect(selection.selected).toEqual([]);
    row(target, 3).dispatchEvent(new MouseEvent('click', { bubbles: true, metaKey: true }));
    flushSync();
    expect(selection.selected).toEqual([3]);
    row(target, 4).dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: true }));
    flushSync();
    expect(selection.selected).toEqual([3, 4]);
    expect(target.querySelector('[aria-label="Select ABC-3"][data-state="checked"]')).not.toBeNull();
    expect(row(target, 4).dataset.selected).toBeDefined();
  });

  it('focused_row_is_marked', async () => {
    const { target } = await open({ issues: [issue(1), issue(2)] });
    selection.focus(2);
    flushSync();
    expect(row(target, 2).getAttribute('aria-current')).toBe('true');
    expect(row(target, 1).getAttribute('aria-current')).toBeNull();
  });

  it('overdue_due_dates_are_marked_unless_done', async () => {
    const done = fixtures.statuses.find((s) => s.category === 'Completed')!;
    const { target } = await open({
      issues: [
        issue(1, { due: { Date: '2001-01-02' } }),
        issue(2, { due: { Date: '2999-01-02' } }),
        issue(3, { due: { Date: '2001-01-02' }, status_id: done.id }),
      ],
    });
    expect(row(target, 1).querySelector('.due')!.hasAttribute('data-overdue')).toBe(true);
    expect(row(target, 2).querySelector('.due')!.hasAttribute('data-overdue')).toBe(false);
    expect(row(target, 3).querySelector('.due')!.hasAttribute('data-overdue')).toBe(false);
  });

  it('publishes_ordered_visible_ids', async () => {
    const issues = [issue(1), issue(2, { status_id: S2.id }), issue(3)];
    const { target } = await open({ issues });
    expect(listState.ids).toEqual([1, 3, 2]);
    target.querySelector<HTMLElement>(`[aria-label="Collapse ${S1.name}"]`)!.click();
    flushSync();
    expect(listState.ids).toEqual([2]);
  });

  it('status_glyph_opens_menu_and_updates_status', async () => {
    const { mock, target } = await open({ issues: [issue(1)] });
    const spy = vi.spyOn(mock.issues, 'update');
    row(target, 1).querySelector<HTMLElement>('button.status')!.click();
    flushSync();
    const item = await vi.waitFor(() => {
      const el = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((e) => e.textContent?.includes(S2.name));
      expect(el).toBeTruthy();
      return el!;
    });
    item.click();
    await vi.waitFor(() => expect(spy).toHaveBeenCalledWith(1, { status_id: S2.id }));
  });

  it('inline_rename_saves_title_on_submit', async () => {
    const { mock, target } = await open({ issues: [issue(1)] });
    const spy = vi.spyOn(mock.issues, 'update');
    listState.editing = 1;
    flushSync();
    const input = target.querySelector<HTMLInputElement>('input')!;
    input.value = 'Renamed';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.form!.requestSubmit();
    await vi.waitFor(() => expect(spy).toHaveBeenCalledWith(1, { title: 'Renamed' }));
    await vi.waitFor(() => expect(listState.editing).toBeNull());
  });

  it('rename_commands_follow_focus_and_editing', () => {
    listState.ids = [1];
    const ids = (id: number | null) => listCommands(ctxFor(id)).map((c) => c.id);
    expect(ids(1)).toContain('list.title.edit');
    expect(ids(null)).not.toContain('list.title.edit');
    expect(ids(1)).not.toContain('list.title.cancel');
    listState.editing = 1;
    expect(ids(1)).toContain('list.title.cancel');
    expect(ids(1)).not.toContain('list.title.edit');
  });

  it('add_in_group_creates_issue_in_that_status', async () => {
    const { mock, target } = await open({ issues: [issue(1)] });
    const spy = vi.spyOn(mock.issues, 'create');
    target.querySelector<HTMLElement>(`[aria-label="Add to ${S1.name}"]`)!.click();
    await vi.waitFor(() => expect(spy).toHaveBeenCalledWith(expect.objectContaining({ status_id: S1.id })));
    await vi.waitFor(() => expect(listState.editing).not.toBeNull());
  });

  it('right_click_opens_menu_of_commands_for_that_row', async () => {
    const { target } = await open({ issues: [issue(1)] });
    row(target, 1).dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    flushSync();
    await vi.waitFor(() => expect([...document.querySelectorAll('[role="menuitem"]')].some((e) => e.textContent?.includes('Rename issue'))).toBe(true));
    expect(selection.focused).toBe(1);
  });

  it('row_shows_labels_project_due_and_estimate', async () => {
    const proj = fixtures.projects[0];
    const labels = [{ id: 501, name: 'bug', color: 'red', color_hex: '#dd3c31', icon: null }];
    const { target } = await open({
      labels,
      issues: [issue(1, { project_id: proj.id, label_ids: [501], due: { Date: '2999-12-03' }, estimate_minutes: 90 })],
    });
    const text = row(target, 1).textContent!;
    for (const part of ['bug', proj.name, 'Dec 3', '1h 30m']) expect(text).toContain(part);
  });

  it('unknown_status_id_renders_under_unknown', async () => {
    const { target } = await open({ issues: [issue(1, { status_id: 9999 })] });
    expect(bands(target).map((b) => b.name)).toEqual(['Unknown']);
    expect(rows(target)).toHaveLength(1);
  });

  it('empty_view_shows_empty_state', async () => {
    const { target } = await open({ issues: [] });
    expect(target.querySelector('.empty')).not.toBeNull();
    expect(rows(target)).toHaveLength(0);
  });

  it('renders_1000_issues_with_median_under_100ms', async () => {
    const many = Array.from({ length: 1000 }, (_, k) => issue(k + 1));
    const times: number[] = [];
    for (let run = 0; run < 5; run++) {
      const { target, ms } = await open({ issues: many, views: viewSeed({ group_by: null }) });
      times.push(ms);
      expect(rows(target).length).toBeLessThan(100);
      expect(target.textContent).toContain('Row 1');
      expect(target.textContent).not.toContain('Row 1000');
      unmount(live!);
      live = null;
      document.body.innerHTML = '';
    }
    expect(times.sort((a, b) => a - b)[2]).toBeLessThan(100);
  });
});
