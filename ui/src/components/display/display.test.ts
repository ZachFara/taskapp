import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import { refreshAll } from '../../lib/stores.svelte';
import type { Issue, PropertyDef, SavedView, ViewDef } from '../../lib/bindings';
import ListView from '../views/ListView.svelte';
import BoardView from '../views/BoardView.svelte';
import TableView from '../views/TableView.svelte';
import DisplayPanel from './DisplayPanel.svelte';

const area: PropertyDef = { key: 'area', name: 'Area', kind: 'Select', options: [{ value: 'Home', color: 'green' }], project_id: null, position: 0 };
const done: PropertyDef = { key: 'done', name: 'Shipped', kind: 'Checkbox', options: [], project_id: null, position: 1 };
const issue = (n: number, over: Partial<Issue> = {}): Issue => ({
  ...fixtures.issues[0], id: n, key: `ABC-${n}`, title: `Row ${n}`, status_id: fixtures.statuses[0].id, priority: 0, project_id: null,
  cycle_id: null, label_ids: [], start_date: null, due: null, estimate_minutes: null, properties: { area: 'Home', done: true }, ...over,
});
const def = (over: Record<string, unknown> = {}, layout: ViewDef['layout'] = 'List'): ViewDef =>
  ({ filter: null, sorts: [], group_by: null, layout, columns: [{ Property: 'area' }], include_archived: false, ...over }) as ViewDef;
const viewOf = (d: ViewDef): SavedView => ({ id: 5, name: 'V', def: d, position: 0, icon: null, color: null });

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});
async function render(comp: Parameters<typeof mount>[0], props: Record<string, unknown>, ready: string, view?: SavedView) {
  const seed = { ...fixtures, propertyDefs: [area, done], issues: [issue(1), issue(2, { properties: {} })], views: view ? [view] : [] } as MockSeed;
  setApi(createMockApi(seed));
  await refreshAll();
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(comp, { target, props });
  flushSync();
  await vi.waitFor(() => expect(target.querySelector(ready)).not.toBeNull(), { interval: 1 });
  await new Promise((r) => setTimeout(r, 10));
  flushSync();
  return target;
}

describe('custom properties in layouts', () => {
  it('list_row_shows_a_select_property_as_a_pill', async () => {
    const v = viewOf(def());
    const t = await render(ListView, { view: v }, 'li[data-key]');
    expect(t.querySelector('li[data-key="ABC-1"]')!.textContent).toContain('Home');
  });
  it('list_shows_only_the_displayed_fields_in_order', async () => {
    const v = viewOf(def({ columns: [{ Property: 'done' }, { Property: 'area' }] }));
    const t = await render(ListView, { view: v }, 'li[data-key]');
    const cells = [...t.querySelectorAll('li[data-key="ABC-1"] [data-field]')].map((c) => c.getAttribute('data-field'));
    expect(cells).toEqual(['P:done', 'P:area']);
  });
  it('compact_density_marks_the_list', async () => {
    const t = await render(ListView, { view: viewOf(def({ density: 'compact' })) }, 'li[data-key]');
    expect(t.querySelector('[data-density="compact"]')).not.toBeNull();
  });
  it('board_card_shows_a_custom_property', async () => {
    const t = await render(BoardView, { view: viewOf(def({}, 'Board')) }, '[data-issue]');
    expect(t.querySelector('[data-issue="ABC-1"]')!.textContent).toContain('Home');
  });
  it('table_column_shows_a_custom_property', async () => {
    const t = await render(TableView, { view: viewOf(def({}, 'Table')) }, 'tbody tr, [role=row]', viewOf(def({}, 'Table')));
    expect(t.textContent).toContain('Home');
  });
});

describe('DisplayPanel', () => {
  const panel = async (d: ViewDef, patches: unknown[]) =>
    render(DisplayPanel, { def: d, onchange: (p: unknown) => patches.push(p) }, '[data-field="P:area"]');
  const click = (el: Element | null) => { (el as HTMLElement).click(); flushSync(); };
  const sw = (t: HTMLElement, label: string) => t.querySelector(`[aria-label="${label}"]`);

  it('lists_every_custom_property_with_a_switch', async () => {
    const t = await panel(def({ columns: [] }), []);
    expect(sw(t, 'Show Area')).not.toBeNull();
    expect(sw(t, 'Show Shipped')).not.toBeNull();
  });
  it('toggling_a_field_appends_it_to_columns', async () => {
    const patches: unknown[] = [];
    const t = await panel(def(), patches);
    click(sw(t, 'Show Shipped'));
    expect(patches).toEqual([{ columns: [{ Property: 'area' }, { Property: 'done' }] }]);
  });
  it('reorder_moves_a_field_down', async () => {
    const patches: unknown[] = [];
    const t = await panel(def({ columns: [{ Property: 'area' }, { Property: 'done' }] }), patches);
    click(sw(t, 'Move Area down'));
    expect(patches).toEqual([{ columns: [{ Property: 'done' }, { Property: 'area' }] }]);
  });
  it('reset_clears_columns', async () => {
    const patches: unknown[] = [];
    const t = await panel(def(), patches);
    click([...t.querySelectorAll('button')].find((b) => b.textContent?.includes('Reset to default')) ?? null);
    expect(patches).toEqual([{ columns: [] }]);
  });
  it('compact_switch_sets_density', async () => {
    const patches: unknown[] = [];
    const t = await panel(def(), patches);
    click(sw(t, 'Compact rows'));
    expect(patches).toEqual([{ density: 'compact' }]);
  });
  it('weekends_option_only_on_dated_layouts', async () => {
    expect(sw(await panel(def(), []), 'Show weekends')).toBeNull();
    unmount(live!); live = null; document.body.innerHTML = '';
    expect(sw(await panel(def({}, 'Calendar'), []), 'Show weekends')).not.toBeNull();
  });
});
