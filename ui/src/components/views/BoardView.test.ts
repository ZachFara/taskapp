import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { api, setApi } from '../../lib/api';
import { createMockApi, type MockSeed } from '../../lib/api.mock';
import type { Issue, Label, PropertyDef, SavedView, Status, ViewDef } from '../../lib/bindings';
import { selection } from '../../lib/selection.svelte';
import { refreshAll } from '../../lib/stores.svelte';
import BoardView from './BoardView.svelte';

const status = (id: number, name: string, category: Status['category'], position: number): Status => ({
  id, name, category, position, color: 'blue', color_hex: '#2b76da', icon: null,
});

const statuses: Status[] = [
  status(1, 'Todo', 'Unstarted', 0),
  status(2, 'Doing', 'Started', 1),
  status(3, 'Finished', 'Completed', 2),
];

const issue = (id: number, status_id: number, extra: Partial<Issue> = {}): Issue => ({
  id, key: `BRD-${id}`, title: `Card ${id}`, description: '', status_id, priority: 0, project_id: null,
  milestone_id: null, parent_id: null, cycle_id: null, label_ids: [], start_date: null, due: null,
  estimate_minutes: null, properties: {}, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z',
  completed_at: null, archived_at: null, ...extra,
});

const seed = (extra: Partial<MockSeed> = {}): MockSeed => ({
  statuses,
  projects: [],
  propertyDefs: [],
  issues: [issue(1, 1), issue(2, 2), issue(3, 3)],
  labels: [],
  views: [],
  ...extra,
});

const board = (group_by: ViewDef['group_by'] = null): ViewDef => ({
  filter: null, sorts: [], group_by, layout: 'Board', columns: [], include_archived: false,
});
const boardView = (group_by: ViewDef['group_by'] = null): SavedView => ({ id: 7, name: 'Board', def: board(group_by), position: 0 });

let cleanups: (() => void)[] = [];
beforeEach(() => {
  localStorage.clear();
  selection.clear();
  selection.focus(null);
});
afterEach(() => {
  cleanups.forEach((c) => c());
  cleanups = [];
  vi.restoreAllMocks();
});

async function render(s: MockSeed, view: SavedView = boardView()) {
  setApi(createMockApi(s));
  await refreshAll();
  const target = document.createElement('div');
  document.body.append(target);
  const app = mount(BoardView, { target, props: { view } });
  flushSync();
  // The board runs the view's own query, so its columns appear once that resolves.
  await vi.waitFor(() => expect(target.querySelector('.board, [role=alert]')).not.toBeNull());
  await new Promise((r) => setTimeout(r, 5));
  flushSync();
  cleanups.push(() => {
    unmount(app);
    target.remove();
  });
  return target;
}

const column = (root: Element, name: string) => root.querySelector(`[data-column="${name}"]`)!;
const card = (root: Element, key: string) => root.querySelector(`[data-issue="${key}"]`)!;
const cardKeys = (root: Element) => [...root.querySelectorAll('[data-issue]')].map((c) => c.getAttribute('data-issue'));

// jsdom has no DataTransfer, so the drag events carry a plain object.
function drag(root: Element, key: string, into: string) {
  const data = new Map<string, string>();
  const dataTransfer = {
    setData: (type: string, value: string) => data.set(type, value),
    getData: (type: string) => data.get(type) ?? '',
    effectAllowed: '',
    dropEffect: '',
  };
  const fire = (el: Element, type: string) => {
    const ev = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(ev, 'dataTransfer', { value: dataTransfer });
    el.dispatchEvent(ev);
  };
  fire(card(root, key), 'dragstart');
  fire(column(root, into), 'dragover');
  fire(column(root, into), 'drop');
  fire(card(root, key), 'dragend');
  flushSync();
}

const columnNames = (root: Element) =>
  [...root.querySelectorAll('[data-column]')].map((c) => c.getAttribute('data-column'));

describe('BoardView', () => {
  it('shows_only_the_filtered_issues_and_hides_excluded_columns', async () => {
    const view: SavedView = {
      ...boardView(),
      def: { ...board(), filter: { Cond: { field: { Fixed: 'StatusCategory' }, op: 'In', value: ['unstarted', 'started'] } } },
    };
    const root = await render(seed(), view);
    expect(columnNames(root)).toEqual(['Todo', 'Doing']);
    expect(cardKeys(root)).toEqual(['BRD-1', 'BRD-2']);
  });

  it('columns_follow_status_positions', async () => {
    const shuffled = [statuses[2], statuses[0], statuses[1]];
    const root = await render(seed({ statuses: shuffled, issues: [] }));
    expect(columnNames(root)).toEqual(['Todo', 'Doing', 'Finished']);
  });

  it('drop_updates_status', async () => {
    const root = await render(seed());
    const update = vi.spyOn(api.issues, 'update');
    drag(root, 'BRD-1', 'Doing');
    expect(update).toHaveBeenCalledTimes(1);
    expect(update).toHaveBeenCalledWith(1, { status_id: 2 });
    expect(column(root, 'Doing').contains(card(root, 'BRD-1'))).toBe(true);
  });

  it('group_by_select_property_columns', async () => {
    const area: PropertyDef = {
      key: 'area', name: 'Area', kind: 'Select', project_id: null, position: 0,
      options: [{ value: 'home' }, { value: 'work' }] as PropertyDef['options'],
    };
    const root = await render(seed({ propertyDefs: [area] }), boardView({ Property: 'area' }));
    expect(columnNames(root)).toEqual(['home', 'work', 'None']);
    const update = vi.spyOn(api.issues, 'update');
    drag(root, 'BRD-1', 'work');
    expect(update).toHaveBeenCalledWith(1, { properties: { area: 'work' } });
  });

  it('drop_on_same_column_calls_no_api', async () => {
    const root = await render(seed());
    const update = vi.spyOn(api.issues, 'update');
    drag(root, 'BRD-1', 'Todo');
    expect(update).not.toHaveBeenCalled();
  });

  it('failed_update_moves_card_back_and_shows_error', async () => {
    const root = await render(seed());
    vi.spyOn(api.issues, 'update').mockRejectedValue({ kind: 'Invalid', message: 'status is archived' });
    drag(root, 'BRD-1', 'Doing');
    await vi.waitFor(() => expect(root.querySelector('[role="alert"]')?.textContent).toContain('status is archived'));
    flushSync();
    expect(column(root, 'Todo').contains(card(root, 'BRD-1'))).toBe(true);
    expect(column(root, 'Doing').contains(card(root, 'BRD-1'))).toBe(false);
  });

  it('empty_column_shows_designed_empty_state', async () => {
    const root = await render(seed({ issues: [issue(1, 1)] }));
    const doing = column(root, 'Doing');
    expect(doing.querySelectorAll('[data-issue]')).toHaveLength(0);
    expect(doing.textContent).toContain('No issues');
    expect(doing.querySelector('svg.contour')?.getAttribute('aria-hidden')).toBe('true');
  });

  it('card_shows_key_title_priority_and_labels', async () => {
    const labels: Label[] = [{ id: 1, name: 'urgent-bug', color: 'red', color_hex: '#dd3c31', icon: null }];
    const root = await render(seed({
      labels,
      issues: [issue(1, 1, { label_ids: [1], priority: 3 })],
    }));
    await vi.waitFor(() => expect(card(root, 'BRD-1').textContent).toContain('urgent-bug'));
    const el = card(root, 'BRD-1');
    expect(el.textContent).toContain('BRD-1');
    expect(el.textContent).toContain('Card 1');
    expect(el.querySelector('[role="img"][data-level="3"]')).not.toBeNull();
  });

  it('click_focuses_card_and_ctrl_click_toggles_selection', async () => {
    const root = await render(seed());
    const c1 = card(root, 'BRD-1');
    const c2 = card(root, 'BRD-2');
    c1.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    flushSync();
    expect(selection.focused).toBe(1);
    expect(c1.getAttribute('aria-selected')).toBe('true');
    expect(c1.getAttribute('data-focused')).toBe('');
    c2.dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
    flushSync();
    expect(selection.selected).toEqual([2]);
    expect(selection.focused).toBe(2);
    expect(c2.getAttribute('aria-selected')).toBe('true');
    expect(c2.getAttribute('data-focused')).toBe('');
    expect(c1.getAttribute('data-focused')).toBeNull();
    c2.dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
    flushSync();
    expect(selection.selected).toEqual([]);
  });

  it('shift_click_selects_the_range_in_reading_order', async () => {
    // Finished is collapsed by default, so the range stays inside the visible columns.
    const root = await render(seed({ issues: [issue(1, 1), issue(2, 1), issue(3, 2)] }));
    card(root, 'BRD-1').dispatchEvent(new MouseEvent('click', { bubbles: true }));
    card(root, 'BRD-3').dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: true }));
    flushSync();
    expect(selection.selected).toEqual([1, 2, 3]);
  });

  it('collapsed_columns_default_to_done_and_persist', async () => {
    const root = await render(seed());
    const finished = column(root, 'Finished');
    expect(finished.querySelectorAll('[data-issue]')).toHaveLength(0);
    expect(finished.querySelector('button[aria-expanded]')?.getAttribute('aria-expanded')).toBe('false');
    (finished.querySelector('button[aria-expanded]') as HTMLButtonElement).click();
    flushSync();
    expect(column(root, 'Finished').querySelectorAll('[data-issue]')).toHaveLength(1);
    cleanups.pop()?.();
    const again = await render(seed());
    expect(column(again, 'Finished').querySelectorAll('[data-issue]')).toHaveLength(1);
  });

  it('hide_empty_columns_option_removes_them', async () => {
    const root = await render(seed({ issues: [issue(1, 1)] }));
    expect(columnNames(root)).toContain('Doing');
    const toggle = root.querySelector('button[aria-label="Hide empty columns"]') as HTMLButtonElement;
    toggle.click();
    flushSync();
    expect(columnNames(root)).not.toContain('Doing');
    expect(columnNames(root)).toContain('Todo');
  });
});
