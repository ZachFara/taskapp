import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { api, setApi } from '../../lib/api';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import type { SavedView, Status, ViewDef } from '../../lib/bindings';
import { refreshAll } from '../../lib/stores.svelte';
import BoardView from './BoardView.svelte';

const board = (group_by: ViewDef['group_by'] = null): ViewDef => ({
  filter: null, sorts: [], group_by, layout: 'Board', columns: [], include_archived: false,
});

const seedWith = (extra: Partial<MockSeed>): MockSeed => ({ ...fixtures, ...extra });
const boardView = (group_by: ViewDef['group_by'] = null): SavedView => ({ id: 1, name: 'Board', def: board(group_by), position: 0 });

let cleanups: (() => void)[] = [];
afterEach(() => {
  cleanups.forEach((c) => c());
  cleanups = [];
  vi.restoreAllMocks();
});

async function render(seed: MockSeed, view: SavedView = boardView()) {
  setApi(createMockApi(seed));
  await refreshAll();
  const target = document.createElement('div');
  document.body.append(target);
  const app = mount(BoardView, { target, props: { view } });
  flushSync();
  cleanups.push(() => {
    unmount(app);
    target.remove();
  });
  return target;
}

const column = (root: Element, name: string) => root.querySelector(`[data-column="${name}"]`)!;
const card = (root: Element, key: string) => root.querySelector(`[data-issue="${key}"]`)!;

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
  it('columns_follow_status_positions', async () => {
    const statuses: Status[] = [
      { id: 1, name: 'Done', category: 'Completed', position: 2, color: '#888' },
      { id: 2, name: 'Todo', category: 'Unstarted', position: 0, color: '#888' },
      { id: 3, name: 'Started', category: 'Started', position: 1, color: '#888' },
    ];
    const root = await render(seedWith({ statuses, issues: [] }));
    expect(columnNames(root)).toEqual(['Todo', 'Started', 'Done']);
  });

  it('drop_updates_status', async () => {
    const root = await render(seedWith({}));
    const update = vi.spyOn(api.issues, 'update');
    drag(root, 'ZF-1', 'Todo');
    expect(update).toHaveBeenCalledWith(1, { status_id: 2 });
    expect(column(root, 'Todo').contains(card(root, 'ZF-1'))).toBe(true);
  });

  it('group_by_select_property_columns', async () => {
    const root = await render(seedWith({}), boardView({ Property: 'area' }));
    expect(columnNames(root)).toEqual(['home', 'work', 'None']);
    const update = vi.spyOn(api.issues, 'update');
    drag(root, 'ZF-1', 'work');
    expect(update).toHaveBeenCalledWith(1, { properties: { area: 'work', effort: 1 } });
  });

  it('drop_on_same_column_calls_no_api', async () => {
    const root = await render(seedWith({}));
    const update = vi.spyOn(api.issues, 'update');
    drag(root, 'ZF-1', 'Backlog');
    expect(update).not.toHaveBeenCalled();
  });

  it('failed_update_moves_card_back_and_shows_error', async () => {
    const root = await render(seedWith({}));
    vi.spyOn(api.issues, 'update').mockRejectedValue({ kind: 'Invalid', message: 'status is archived' });
    drag(root, 'ZF-1', 'Todo');
    await vi.waitFor(() => expect(root.querySelector('[role="alert"]')?.textContent).toContain('status is archived'));
    flushSync();
    expect(column(root, 'Backlog').contains(card(root, 'ZF-1'))).toBe(true);
    expect(column(root, 'Todo').contains(card(root, 'ZF-1'))).toBe(false);
  });

  it('empty_status_column_still_renders', async () => {
    const statuses = [...fixtures.statuses, { id: 6, name: 'Icebox', category: 'Backlog', position: 5, color: '#888' } as Status];
    const root = await render(seedWith({ statuses }));
    expect(columnNames(root)).toContain('Icebox');
    expect(column(root, 'Icebox').querySelectorAll('[data-issue]')).toHaveLength(0);
  });

  it('card_shows_key_title_priority_and_labels', async () => {
    const labels = [{ id: 1, name: 'bug', color: '#f00' }];
    const issues = fixtures.issues.map((i) => (i.id === 1 ? { ...i, label_ids: [1], priority: 3 } : i));
    const root = await render(seedWith({ labels, issues }));
    await vi.waitFor(() => expect(card(root, 'ZF-1').textContent).toContain('bug'));
    const text = card(root, 'ZF-1').textContent ?? '';
    expect(text).toContain('ZF-1');
    expect(text).toContain('Fixture task 1');
    expect(text).toContain('P3');
  });

  it('jk_enter_s_work_on_cards', async () => {
    history.replaceState(null, '', '/');
    const root = await render(seedWith({}));
    const key = (k: string) => window.dispatchEvent(new KeyboardEvent('keydown', { key: k }));
    const sel = () => root.querySelector<HTMLElement>('[aria-selected="true"]')?.dataset.issue;
    const first = sel();
    expect(first).toBeDefined();
    key('j');
    flushSync();
    expect(sel()).not.toBe(first);
    key('k');
    flushSync();
    expect(sel()).toBe(first);
    const from = card(root, first!).closest('[data-column]')!.getAttribute('data-column');
    key('s');
    await vi.waitFor(() => expect(card(root, first!).closest('[data-column]')!.getAttribute('data-column')).not.toBe(from));
    key('Enter');
    expect(location.pathname).toBe(`/issue/${first}`);
  });
});
