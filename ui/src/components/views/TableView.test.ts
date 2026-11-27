import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, setApi } from '../../lib/api';
import type { Issue, PropertyDef, SavedView, Status } from '../../lib/bindings';
import { createMockApi } from '../../lib/api.mock';
import { attachKeys } from '../../lib/commands';
import { selection } from '../../lib/selection.svelte';
import { currentViewId } from '../../lib/stores.svelte';
import TableView from './TableView.svelte';

// The test owns its data: nothing here depends on the shared fixtures.
const statuses: Status[] = [
  { id: 1, name: 'Todo', category: 'Unstarted', position: 0, color: 'gray', color_hex: '#6e7c75', icon: null },
  { id: 2, name: 'Doing', category: 'Started', position: 1, color: 'blue', color_hex: '#2b76da', icon: null },
  { id: 3, name: 'Done', category: 'Completed', position: 2, color: 'green', color_hex: '#2e8a43', icon: null },
];
const propertyDefs: PropertyDef[] = [
  { key: 'effort', name: 'Effort', kind: 'Number', options: [], project_id: null, position: 0 },
  {
    key: 'area',
    name: 'Area',
    kind: 'Select',
    options: [
      { value: 'Ops', color: 'blue' },
      { value: 'Web', color: 'purple' },
    ],
    project_id: null,
    position: 1,
  },
  { key: 'reviewed', name: 'Reviewed', kind: 'Checkbox', options: [], project_id: null, position: 2 },
];
const issue = (id: number, key: string, title: string, over: Partial<Issue> = {}): Issue => ({
  id,
  key,
  title,
  description: '',
  status_id: 1,
  priority: 0,
  project_id: null,
  milestone_id: null,
  parent_id: null,
  cycle_id: null,
  label_ids: [],
  start_date: null,
  due: null,
  estimate_minutes: null,
  properties: {},
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  completed_at: null,
  archived_at: null,
  ...over,
});
const issues: Issue[] = [
  issue(1, 'ABC-1', 'Write spec', { status_id: 1, priority: 2, properties: { effort: 1, area: 'Ops', reviewed: false } }),
  issue(2, 'ABC-2', 'Ship build', { status_id: 2, priority: 1, properties: { effort: 3, area: 'Web', reviewed: true } }),
  issue(3, 'ABC-3', 'Archive docs', { status_id: 3, priority: 0, properties: {} }),
];
const columns = [{ Fixed: 'Title' as const }, { Property: 'effort' }, { Property: 'gone' }, { Property: 'area' }];
const viewOf = (over: Partial<SavedView['def']> = {}): SavedView => ({
  id: 1,
  name: 'All',
  position: 0,
  icon: null,
  color: null,
  def: { filter: null, sorts: [], group_by: null, layout: 'Table', columns, include_archived: false, ...over },
});
const seedOf = (over: { view?: SavedView; issues?: Issue[] } = {}) => ({
  statuses,
  projects: [],
  propertyDefs,
  issues: over.issues ?? issues,
  views: [over.view ?? viewOf()],
});

let target: HTMLElement;
let app: ReturnType<typeof mount>;
let mock: ReturnType<typeof createMockApi>;
let detachKeys: () => void;

const flush = async () => {
  await tick();
  flushSync();
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const cell = (key: string, col: string) => target.querySelector<HTMLElement>(`tr[data-key="${key}"] td[data-col="${col}"]`)!;
const activeCell = () => {
  const td = target.querySelector<HTMLElement>('td[data-active]')!;
  return `${td.closest('tr')!.dataset.key}|${td.dataset.col}`;
};
const rowKeys = () => [...target.querySelectorAll<HTMLElement>('tbody tr[data-key]')].map((tr) => tr.dataset.key);
const headerButton = (col: string, text: string) =>
  [...target.querySelectorAll<HTMLButtonElement>(`th[data-col="${col}"] button`)].find((b) => b.textContent?.trim() === text)!;
const press = (el: EventTarget, key: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
const type = (input: HTMLInputElement, text: string) => {
  input.value = text;
  input.dispatchEvent(new Event('input', { bubbles: true }));
};
// Enter inside a text field submits its form; jsdom does not simulate implicit submission, so do what the browser does.
const submitEditor = (input: HTMLInputElement) => input.form!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
const alertText = () => target.querySelector('[role="alert"]')?.textContent?.trim() ?? '';

// The screen reads its view from the store after the first refresh, so a view under test must also be in the seed.
async function open(view?: SavedView, { rows = true } = {}) {
  target = document.body.appendChild(document.createElement('div'));
  app = mount(TableView, { target, props: { view: view ?? viewOf() } });
  await flush();
  if (rows) await vi.waitFor(() => expect(target.querySelector('tbody tr[data-key]')).not.toBeNull());
}
const seeded = (view: SavedView) => {
  mock = createMockApi(seedOf({ view }));
  setApi(mock);
};

async function startEdit(key: string, col: string) {
  cell(key, col).click();
  press(window, 'Enter');
  await flush();
  return cell(key, col).querySelector<HTMLInputElement>('input')!;
}

beforeEach(() => {
  mock = createMockApi(seedOf());
  setApi(mock);
  currentViewId.value = 1;
  selection.clear();
  selection.focus(null);
  detachKeys = attachKeys(() => ({
    route: '/view/1/table',
    selection: selection.selected,
    focusedIssueId: selection.focused,
    openIssueId: null,
    api: mock,
    navigate: () => {},
  }));
});

afterEach(async () => {
  detachKeys();
  await unmount(app);
  target.remove();
  vi.restoreAllMocks();
});

describe('TableView', () => {
  it('updates_after_an_issue_is_created_outside_the_table', async () => {
    await open();
    const made = await api.issues.create({ title: 'New row', status_id: 1 });
    await vi.waitFor(() => expect(rowKeys()).toContain(made.key));
  });

  it('sort_click_header_orders_rows_then_reverses', async () => {
    await open();
    expect(rowKeys()).toEqual(['ABC-1', 'ABC-2', 'ABC-3']);
    headerButton('fixed:Title', 'Title').click();
    await flush();
    expect(rowKeys()).toEqual(['ABC-3', 'ABC-2', 'ABC-1']);
    headerButton('fixed:Title', 'Title').click();
    await flush();
    expect(rowKeys()).toEqual(['ABC-1', 'ABC-2', 'ABC-3']);
  });

  it('hide_column_persists', async () => {
    const save = vi.spyOn(mock.views, 'save');
    await open();
    target.querySelector<HTMLButtonElement>('button[aria-label="Remove column Effort"]')!.click();
    await vi.waitFor(() => expect(save).toHaveBeenCalled());
    expect(save.mock.calls[0][2].columns).toEqual([{ Fixed: 'Title' }, { Property: 'gone' }, { Property: 'area' }]);
    unmount(app);
    target.remove();
    const saved = (await mock.views.list())[0];
    await open(saved);
    expect(target.querySelector('th[data-col="prop:effort"]')).toBeNull();
    expect(target.querySelector('th[data-col="prop:area"]')).not.toBeNull();
  });

  it('edit_number_property_saves', async () => {
    const save = vi.spyOn(mock.properties, 'setValue');
    await open();
    const input = await startEdit('ABC-1', 'prop:effort');
    type(input, '5');
    submitEditor(input);
    await vi.waitFor(() => expect(save).toHaveBeenCalledWith(1, 'effort', 5));
    await vi.waitFor(() => expect(cell('ABC-1', 'prop:effort').textContent?.trim()).toBe('5'));
  });

  it('edit_title_calls_issue_update', async () => {
    const update = vi.spyOn(mock.issues, 'update');
    await open();
    const input = await startEdit('ABC-2', 'fixed:Title');
    type(input, 'Ship it');
    submitEditor(input);
    await vi.waitFor(() => expect(update).toHaveBeenCalledWith(2, { title: 'Ship it' }));
  });

  it('non_numeric_number_is_rejected_before_api', async () => {
    const save = vi.spyOn(mock.properties, 'setValue');
    await open();
    const input = await startEdit('ABC-1', 'prop:effort');
    type(input, 'abc');
    submitEditor(input);
    await flush();
    expect(save).not.toHaveBeenCalled();
    expect(alertText()).toContain('"abc" is not a number');
  });

  it('invalid_value_reverts_with_message', async () => {
    const save = vi.spyOn(mock.properties, 'setValue').mockRejectedValueOnce({ kind: 'Invalid', message: 'effort is out of range' });
    await open();
    const input = await startEdit('ABC-1', 'prop:effort');
    type(input, '7');
    submitEditor(input);
    await vi.waitFor(() => expect(alertText()).toBe('effort is out of range'));
    expect(save).toHaveBeenCalledWith(1, 'effort', 7);
    expect(cell('ABC-1', 'prop:effort').querySelector('input')).toBeNull();
    expect(cell('ABC-1', 'prop:effort').textContent?.trim()).toBe('1');
  });

  it('esc_while_editing_restores_without_api_call', async () => {
    const save = vi.spyOn(mock.properties, 'setValue');
    const update = vi.spyOn(mock.issues, 'update');
    await open();
    const input = await startEdit('ABC-1', 'prop:effort');
    type(input, '9');
    press(input, 'Escape');
    await flush();
    expect(cell('ABC-1', 'prop:effort').querySelector('input')).toBeNull();
    expect(cell('ABC-1', 'prop:effort').textContent?.trim()).toBe('1');
    expect(save).not.toHaveBeenCalled();
    expect(update).not.toHaveBeenCalled();
  });

  it('arrow_and_hjkl_navigation', async () => {
    await open();
    cell('ABC-1', 'fixed:Title').click();
    await flush();
    expect(activeCell()).toBe('ABC-1|fixed:Title');
    press(window, 'ArrowRight');
    await flush();
    expect(activeCell()).toBe('ABC-1|prop:effort');
    press(window, 'ArrowDown');
    await flush();
    expect(activeCell()).toBe('ABC-2|prop:effort');
    press(window, 'k');
    press(window, 'ArrowUp');
    await flush();
    expect(activeCell()).toBe('ABC-1|prop:effort');
    press(window, 'ArrowLeft');
    press(window, 'h');
    press(window, 'h');
    await flush();
    expect(activeCell()).toBe('ABC-1|fixed:Title');
  });

  it('add_column_updates_view_def', async () => {
    const save = vi.spyOn(mock.views, 'save');
    await open();
    target.querySelector<HTMLButtonElement>('button[aria-label="Add field"]')!.click();
    await flush();
    await vi.waitFor(() => expect(document.querySelector('[aria-label="Show Reviewed"]')).not.toBeNull());
    document.querySelector<HTMLButtonElement>('[aria-label="Show Reviewed"]')!.click();
    await vi.waitFor(() =>
      expect(save).toHaveBeenCalledWith(
        1,
        'All',
        expect.objectContaining({
          columns: [...columns, { Property: 'reviewed' }],
        }),
      ),
    );
  });

  it('publishes_its_rows_and_layout_to_the_selection_store', async () => {
    await open();
    expect(selection.layout).toBe('table');
    expect(selection.columns.flat()).toEqual([1, 2, 3]);
  });

  it('add_field_is_a_button_at_the_end_of_the_header_row', async () => {
    await open();
    const add = target.querySelector<HTMLButtonElement>('button[aria-label="Add field"]')!;
    expect(add.closest('th')).toBe(target.querySelector('thead th:last-child'));
  });

  it('column_with_deleted_def_is_skipped', async () => {
    await open();
    const heads = [...target.querySelectorAll<HTMLElement>('thead th[data-col]')].map((th) => th.dataset.col);
    expect(heads).toEqual(['fixed:Title', 'prop:effort', 'prop:area']);
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });

  it('jk_and_enter_work_from_anywhere', async () => {
    await open();
    cell('ABC-1', 'fixed:Title').click();
    press(window, 'j');
    flushSync();
    expect(activeCell()).toBe('ABC-2|fixed:Title');
    press(window, 'k');
    flushSync();
    expect(activeCell()).toBe('ABC-1|fixed:Title');
    press(window, 'Enter');
    await flush();
    expect(cell('ABC-1', 'fixed:Title').querySelector('input')).not.toBeNull();
  });

  it('a_view_without_columns_shows_the_default_set', async () => {
    seeded(viewOf({ columns: [] }));
    await open(viewOf({ columns: [] }));
    const heads = [...target.querySelectorAll('thead th:not(.add-col)')].map((th) => th.textContent?.trim().split(/\s+/)[0]);
    expect(heads).toEqual(['Key', 'Priority', 'Title', 'Status', 'Project', 'Labels', 'Due', 'Estimate', 'Checklist']);
  });

  it('custom_property_column_renders_by_type', async () => {
    seeded(viewOf({ columns: [...columns, { Property: 'reviewed' }] }));
    await open(viewOf({ columns: [...columns, { Property: 'reviewed' }] }));
    const effort = cell('ABC-2', 'prop:effort');
    expect(effort.textContent?.trim()).toBe('3');
    expect(effort.dataset.kind).toBe('Number');
    expect(effort.classList.contains('num')).toBe(true);
    const area = cell('ABC-2', 'prop:area');
    expect(area.dataset.kind).toBe('Select');
    expect(area.querySelector('.pill')?.textContent?.trim()).toBe('Web');
    expect(area.querySelector('.pill')?.getAttribute('style') ?? '').toContain('--tint: var(--c-purple)');
    expect(cell('ABC-2', 'prop:reviewed').querySelector('[role="checkbox"]')?.getAttribute('aria-checked')).toBe('true');
  });

  it('selection_follows_click_and_toggles_with_x', async () => {
    await open();
    cell('ABC-2', 'fixed:Title').click();
    await flush();
    expect(selection.focused).toBe(2);
    press(window, 'x');
    await flush();
    expect(selection.selected).toEqual([2]);
    press(window, 'x');
    expect(selection.selected).toEqual([]);
  });

  it('groups_render_legend_bands_by_status', async () => {
    seeded(viewOf({ group_by: { Fixed: 'Status' } }));
    await open(viewOf({ group_by: { Fixed: 'Status' } }));
    const bands = [...target.querySelectorAll<HTMLElement>('.band .name')].map((n) => n.textContent?.trim());
    expect(bands).toEqual(['Todo', 'Doing', 'Done']);
    expect(rowKeys()).toEqual(['ABC-1', 'ABC-2', 'ABC-3']);
  });

  it('empty_view_shows_designed_empty_state', async () => {
    mock = createMockApi(seedOf({ issues: [] }));
    setApi(mock);
    await open(undefined, { rows: false });
    await vi.waitFor(() => expect(target.textContent).toContain('No issues in this view'));
  });
});
