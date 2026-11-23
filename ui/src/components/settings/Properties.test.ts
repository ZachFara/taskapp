import { describe, expect, it } from 'vitest';
import Properties from './Properties.svelte';
import { clickLabel, field, mountSettings, pick, press, setField } from './testing';

async function add(target: Element, key: string, kind = 'Text') {
  await setField(target, 'New property name', key.toUpperCase());
  await setField(target, 'New property key', key);
  if (kind !== 'Text') {
    await pick(target, 'New property kind', kind);
  }
  await press(target, 'Add property');
}

describe('Properties', () => {
  it('kind and scope are Select pickers with a chevron, not bare buttons', async () => {
    const { target } = await mountSettings(Properties);
    expect(field(target, 'New property kind').classList.contains('cx-select')).toBe(true);
    expect(field(target, 'New property scope').classList.contains('cx-select')).toBe(true);
  });

  it('adds a property once', async () => {
    const { target, calls } = await mountSettings(Properties);
    await add(target, 'wobble');
    expect(calls.filter((c) => c === 'properties.createDef')).toHaveLength(1);
    expect(field(target, 'Name of WOBBLE')).toBeTruthy();
  });

  it('refuses a malformed key without calling the api', async () => {
    const { target, calls } = await mountSettings(Properties);
    await setField(target, 'New property key', 'Bad Key');
    await press(target, 'Add property');
    expect(calls).not.toContain('properties.createDef');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/lowercase/);
  });

  it('renames inline with one call', async () => {
    const { target, calls } = await mountSettings(Properties);
    await add(target, 'wobble');
    await setField(target, 'Name of WOBBLE', 'Size');
    expect(calls.filter((c) => c === 'properties.renameDef')).toHaveLength(1);
  });

  it('change_kind_conflict_lists_keys', async () => {
    const conflict = async () => {
      throw { kind: 'Conflict', message: 'values do not fit Number: ABC-1, ABC-4' };
    };
    const { target } = await mountSettings(Properties, { properties: { changeKind: conflict } });
    await add(target, 'wobble');
    await pick(target, 'Kind of WOBBLE', 'Number');
    expect(target.querySelector('[role="alert"]')?.textContent).toContain('ABC-1, ABC-4');
  });

  it('move_up_saves_the_new_order', async () => {
    const { target, calls, api } = await mountSettings(Properties);
    await add(target, 'aaa');
    await add(target, 'bbb');
    await clickLabel('Move BBB up');
    expect(calls.filter((c) => c === 'properties.reorder')).toHaveLength(1);
    const keys = (await api.properties.listDefs()).map((d) => d.key);
    expect(keys.indexOf('bbb')).toBeLessThan(keys.indexOf('aaa'));
  });

  it('delete_property_requires_confirm', async () => {
    const { target, calls } = await mountSettings(Properties);
    await add(target, 'wobble');
    await clickLabel('Delete WOBBLE');
    expect(calls).not.toContain('properties.deleteDef');
    await press(document.body, 'Delete property');
    expect(calls.filter((c) => c === 'properties.deleteDef')).toHaveLength(1);
  });

  it('scope can be set to a project', async () => {
    const { target, calls, api } = await mountSettings(Properties);
    await add(target, 'wobble');
    const project = (await api.projects.list())[0];
    await pick(target, 'Scope of WOBBLE', project.name);
    expect(calls.filter((c) => c === 'properties.setScope')).toHaveLength(1);
    expect((await api.properties.listDefs()).find((d) => d.key === 'wobble')?.project_id).toBe(project.id);
  });

  it('select options can be added, recoloured, renamed and removed', async () => {
    const { target, api } = await mountSettings(Properties);
    await add(target, 'stage', 'Select');
    const opts = async () => (await api.properties.listDefs()).find((d) => d.key === 'stage')!.options;
    await setField(target, 'New option of STAGE', 'Draft');
    await press(field(target, 'New option of STAGE').closest('form')!, 'Add option');
    expect(await opts()).toEqual([{ value: 'Draft', color: 'gray' }]);
    await clickLabel('Colour of option Draft');
    await clickLabel('green');
    expect((await opts())[0].color).toBe('green');
    await setField(target, 'Option Draft', 'Final');
    expect((await opts())[0].value).toBe('Final');
    await clickLabel('Remove option Final');
    expect(await opts()).toEqual([]);
  });
});
