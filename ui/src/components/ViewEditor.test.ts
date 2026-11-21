import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import type { SavedView } from '../lib/bindings';
import ViewEditor, { fixedFields, fixedKind, opsFor, toFilter } from './ViewEditor.svelte';

const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const q = <T extends Element = HTMLElement>(s: string, i = 0) => document.querySelectorAll<T>(s)[i];
const click = async (label: string) => {
  [...document.querySelectorAll('button')].find((b) => b.textContent === label)!.click();
  await settle();
};
const set = async (el: HTMLSelectElement | HTMLInputElement, value: string, type = 'change') => {
  el.value = value;
  el.dispatchEvent(new Event(type, { bubbles: true }));
  await settle();
};
const optionsOf = (sel: HTMLSelectElement) => [...sel.options].map((o) => o.value);

let mock: Api;
let app: ReturnType<typeof mount>;
async function open(view: SavedView | null = null) {
  mock = createMockApi(fixtures);
  setApi(mock);
  let saved: SavedView | undefined;
  app = mount(ViewEditor, { target: document.body, props: { view, onsaved: (v: SavedView) => (saved = v) } });
  await settle();
  return () => saved;
}
afterEach(() => unmount(app));

describe('ViewEditor', () => {
  it('build_nested_filter_and_save', async () => {
    const saved = await open();
    await set(q<HTMLInputElement>('[aria-label=Name]'), 'Mine', 'input');
    await click('Add condition');
    await set(q<HTMLSelectElement>('[aria-label=Field]'), 'F:Priority');
    await set(q<HTMLSelectElement>('[aria-label=Operator]'), 'Gte');
    await set(q<HTMLInputElement>('[aria-label=Value]'), '2', 'input');
    await click('Add group');
    await set(q<HTMLSelectElement>('[aria-label=Match]', 1), 'Or');
    await click('Add condition'); // the nested group's own button is the second one
    await settle();
    document.querySelectorAll<HTMLButtonElement>('button').forEach((b) => b.textContent === 'Add condition' && b.parentElement !== q('.group') && b.click());
    await settle();
    await set(q<HTMLSelectElement>('[aria-label=Field]', 1), 'P:area');
    await set(q<HTMLSelectElement>('[aria-label=Operator]', 1), 'In');
    await set(q<HTMLSelectElement>('[aria-label=Value]'), 'home');
    await click('Save view');
    const f = saved()!.def.filter!;
    expect(f.And![0]).toEqual({ Cond: { field: { Fixed: 'Priority' }, op: 'Gte', value: 2 } });
    expect(f.And![1].Or![0].Cond).toMatchObject({ field: { Property: 'area' }, op: 'In' });
    expect((await mock.views.list()).map((v) => v.name)).toEqual(['Mine']);
  });

  it('operators_match_field_type', async () => {
    expect(opsFor('Date')).toEqual(expect.arrayContaining(['Lt', 'Gt']));
    expect(opsFor('Select')).toEqual(expect.arrayContaining(['In']));
    expect(opsFor('Text')).not.toContain('Lt');
    expect(opsFor('Select')).not.toContain('Lt');
    expect(fixedKind('DueDate')).toBe('Date');
    expect(fixedFields).toContain('Project');
    await open();
    await click('Add condition');
    await set(q<HTMLSelectElement>('[aria-label=Field]'), 'F:DueDate');
    expect(optionsOf(q<HTMLSelectElement>('[aria-label=Operator]'))).toEqual(opsFor('Date'));
    expect(q('[aria-label=Value]').getAttribute('type')).toBe('date');
    await set(q<HTMLSelectElement>('[aria-label=Field]'), 'P:area');
    expect(optionsOf(q<HTMLSelectElement>('[aria-label=Operator]'))).toContain('In');
    expect(optionsOf(q<HTMLSelectElement>('[aria-label=Operator]'))).not.toContain('Gt');
  });

  it('layout_switch_persists', async () => {
    const saved = await open();
    await set(q<HTMLInputElement>('[aria-label=Name]'), 'L', 'input');
    await set(q<HTMLSelectElement>('[aria-label=Layout]'), 'Board');
    await click('Save view');
    expect((await mock.views.list())[0].def.layout).toBe('Board');
    expect(saved()!.def.layout).toBe('Board');
  });

  it('update_and_save_as_new', async () => {
    const v = await createMockApi(fixtures).views.save(null, 'Old', { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
    await open(v);
    await mock.views.save(null, 'Old', v.def); // the editor's mock has its own store
    await set(q<HTMLInputElement>('[aria-label=Name]'), 'New', 'input');
    await click('Save as new');
    expect((await mock.views.list()).map((x) => x.name)).toEqual(['Old', 'New']);
  });

  it('saving_a_filter_with_an_empty_group_drops_the_group', async () => {
    const saved = await open();
    await set(q<HTMLInputElement>('[aria-label=Name]'), 'E', 'input');
    await click('Add group');
    await click('Save view');
    expect(saved()!.def.filter).toBeNull();
    expect(toFilter({ kind: 'group', op: 'And', not: false, items: [
      { kind: 'group', op: 'Or', not: false, items: [] },
      { kind: 'cond', field: { Fixed: 'Title' }, op: 'IsEmpty', value: 'x' },
    ] })).toEqual({ And: [{ Cond: { field: { Fixed: 'Title' }, op: 'IsEmpty', value: null } }] });
  });

  it('requires_a_name', async () => {
    await open();
    await click('Save view');
    expect(q('[role=alert]').textContent).toBe('Name is required');
    expect(await mock.views.list()).toEqual([]);
  });
});
