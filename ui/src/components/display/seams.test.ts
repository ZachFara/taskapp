import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import { refreshAll } from '../../lib/stores.svelte';
import type { Issue, PropertyDef, SavedView, ViewDef } from '../../lib/bindings';
import ListView from '../views/ListView.svelte';
import TableView from '../views/TableView.svelte';
import DisplayPanel from './DisplayPanel.svelte';
import { templates } from './templatesApi';

const area: PropertyDef = { key: 'area', name: 'Area', kind: 'Select', options: [{ value: 'Home', color: '#4caf50' }], project_id: null, position: 0 };
const issue = (n: number, over: Partial<Issue> = {}): Issue => ({
  ...fixtures.issues[0], id: n, key: `ABC-${n}`, title: `Row ${n}`, status_id: fixtures.statuses[0].id, priority: 0, project_id: null,
  cycle_id: null, label_ids: [], start_date: null, due: null, estimate_minutes: null, parent_id: null, properties: { area: 'Home' }, ...over,
});
const def = (over: Record<string, unknown> = {}, layout: ViewDef['layout'] = 'List'): ViewDef =>
  ({ filter: null, sorts: [], group_by: null, layout, columns: [], include_archived: false, ...over }) as ViewDef;
const viewOf = (d: ViewDef): SavedView => ({ id: 5, name: 'V', def: d, position: 0, icon: null, color: null, template_id: null });

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});
async function render(comp: Parameters<typeof mount>[0], props: Record<string, unknown>, ready: string, issues = [issue(1), issue(2)], view?: SavedView) {
  const seed = { ...fixtures, propertyDefs: [area], issues, views: view ? [view] : [] } as MockSeed;
  setApi(createMockApi(seed));
  await refreshAll();
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(comp, { target, props });
  flushSync();
  await vi.waitFor(() => expect(target.querySelector(ready)).not.toBeNull(), { interval: 1 });
  await new Promise((r) => setTimeout(r, 20));
  flushSync();
  return target;
}
const click = (el: Element | null) => { (el as HTMLElement).click(); flushSync(); };

describe('display panel sections', () => {
  const panel = (d: ViewDef, patches: unknown[] = []) => render(DisplayPanel, { def: d, onchange: (p: unknown) => patches.push(p) }, '[data-field="P:area"]');
  it('custom_properties_have_their_own_section_with_a_type_icon', async () => {
    const t = await panel(def({ columns: [{ Fixed: 'Priority' }] }));
    const sec = t.querySelector('[data-section="Custom properties"]')!;
    expect(sec.querySelector('[data-field="P:area"]')).not.toBeNull();
    expect(sec.querySelector('[data-field="P:area"] svg')).not.toBeNull();
    expect(t.querySelector('[data-section="Shown"] [data-field="F:Priority"]')).not.toBeNull();
    expect(t.querySelector('[data-section="Hidden"] [data-field="F:Status"]')).not.toBeNull();
  });
  it('hidden_built_ins_are_alphabetical', async () => {
    const t = await panel(def({ columns: [] }));
    const names = [...t.querySelectorAll('[data-section="Hidden"] .name')].map((n) => n.textContent!);
    expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b)));
  });
  it('lists_the_new_built_in_fields', async () => {
    const t = await panel(def({ columns: [] }));
    for (const f of ['Milestone', 'Parent', 'SubIssueProgress', 'RelationCount', 'CommentCount']) expect(t.querySelector(`[data-field="F:${f}"]`), f).not.toBeNull();
  });
  it('board_panel_edits_card_fields', async () => {
    const patches: unknown[] = [];
    const t = await panel(def({ card_fields: [{ Fixed: 'Priority' }] }, 'Board'), patches);
    click(t.querySelector('[aria-label="Show Area"]'));
    expect(patches).toEqual([{ card_fields: [{ Fixed: 'Priority' }, { Property: 'area' }] }]);
  });
  it('move_commands_reorder_through_events', async () => {
    const patches: unknown[] = [];
    const t = await panel(def({ columns: [{ Fixed: 'Priority' }, { Fixed: 'Status' }] }), patches);
    const row = t.querySelector<HTMLElement>('[data-field="F:Priority"]')!;
    row.focus();
    window.dispatchEvent(new CustomEvent('command:view.field.down'));
    flushSync();
    expect(patches).toEqual([{ columns: [{ Fixed: 'Status' }, { Fixed: 'Priority' }] }]);
  });
});

describe('list honours the stored display settings', () => {
  it('key_priority_status_never_render_twice', async () => {
    const t = await render(ListView, { view: viewOf(def({ columns: [{ Fixed: 'Key' }, { Fixed: 'Priority' }, { Fixed: 'Status' }, { Property: 'area' }] })) }, 'li[data-key]');
    const r = t.querySelector('li[data-key="ABC-1"]')!;
    expect(r.textContent!.split('ABC-1').length - 1).toBe(1);
    expect(r.querySelector('[data-field="F:Key"], [data-field="F:Priority"], [data-field="F:Status"]')).toBeNull();
  });
  it('hex_option_colours_tint_the_pill', async () => {
    const t = await render(ListView, { view: viewOf(def({ columns: [{ Property: 'area' }] })) }, 'li[data-key]');
    expect(t.querySelector('li[data-key="ABC-1"] [data-field="P:area"] [data-tinted]')).not.toBeNull();
  });
  it('comment_count_renders_from_the_stats', async () => {
    const t = await render(ListView, { view: viewOf(def({ columns: [{ Fixed: 'CommentCount' }] })) }, 'li[data-key]');
    expect(t.querySelector('li[data-key="ABC-1"] [data-field="F:CommentCount"]')!.textContent!.trim()).toBe('0');
  });
  it('show_empty_groups_lists_statuses_without_issues', async () => {
    const group = { Fixed: 'Status' };
    const off = await render(ListView, { view: viewOf(def({ group_by: group })) }, 'li[data-key]');
    const n = off.querySelectorAll('.head').length;
    unmount(live!); live = null; document.body.innerHTML = '';
    const on = await render(ListView, { view: viewOf(def({ group_by: group, show_empty_groups: true })) }, 'li[data-key]');
    expect(on.querySelectorAll('.head').length).toBe(fixtures.statuses.length);
    expect(n).toBeLessThan(fixtures.statuses.length);
  });
  const kids = [issue(1), issue(2, { parent_id: 1 }), issue(3)];
  it('nest_indents_children_under_a_parent_with_a_disclosure', async () => {
    const t = await render(ListView, { view: viewOf(def({ show_sub_issues: 'nest' })) }, 'li[data-key]', kids);
    expect(t.querySelector('li[data-key="ABC-2"]')!.getAttribute('data-depth')).toBe('1');
    expect(t.querySelector('li[data-key="ABC-1"] [aria-label="Collapse sub-issues"]')).not.toBeNull();
    click(t.querySelector('li[data-key="ABC-1"] [aria-label="Collapse sub-issues"]'));
    await vi.waitFor(() => expect(t.querySelector('li[data-key="ABC-2"]')).toBeNull());
  });
  it('flat_shows_every_issue_unindented', async () => {
    const t = await render(ListView, { view: viewOf(def({ show_sub_issues: 'flat' })) }, 'li[data-key]', kids);
    expect(t.querySelector('li[data-key="ABC-2"]')!.getAttribute('data-depth')).toBeNull();
  });
});

describe('table', () => {
  it('estimate_cells_read_as_durations_not_minutes', async () => {
    const t = await render(TableView, { view: viewOf(def({ columns: [{ Fixed: 'Estimate' }] }, 'Table')) }, 'tbody tr[data-key]', [issue(1, { estimate_minutes: 120 })], viewOf(def({ columns: [{ Fixed: 'Estimate' }] }, 'Table')));
    expect(t.querySelector('tr[data-key="ABC-1"] td[data-col="fixed:Estimate"]')!.textContent!.trim()).toBe('2h');
  });
  it('header_plus_opens_the_display_panel', async () => {
    const v = viewOf(def({ columns: [{ Fixed: 'Estimate' }] }, 'Table'));
    const t = await render(TableView, { view: v }, 'tbody tr[data-key]', [issue(1)], v);
    click(t.querySelector('[aria-label="Add field"]'));
    await vi.waitFor(() => expect(document.querySelector('[aria-label="Display options"]')).not.toBeNull());
  });
});

describe('templates api', () => {
  it('uses_the_generated_names_and_lists_the_seeded_templates', async () => {
    setApi(createMockApi());
    const list = await templates().list();
    expect(list.map((x) => x.name)).toEqual(['Status board', 'Planning table', 'This week']);
  });
});
