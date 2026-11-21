import { describe, expect, it } from 'vitest';
import Properties from './Properties.svelte';
import { mountSettings, press, setField } from './testing';

describe('Properties', () => {
  it('change_kind_conflict_lists_keys', async () => {
    const conflict = async () => {
      throw { kind: 'Conflict', message: 'values do not fit Number: ZF-1, ZF-4' };
    };
    const { target } = await mountSettings(Properties, { properties: { changeKind: conflict } });
    await setField(target, 'Property', 'area');
    await setField(target, 'New kind', 'Number');
    await press(target, 'Change kind');
    expect(target.querySelector('[role="alert"]')?.textContent).toContain('ZF-1, ZF-4');
  });

  it('delete_property_requires_confirm', async () => {
    const { target, calls } = await mountSettings(Properties);
    await setField(target, 'Property', 'effort');
    await press(target, 'Delete');
    const before = calls.length;
    await press(target, 'Cancel');
    expect(calls.slice(before)).toEqual([]);
    expect(calls).not.toContain('properties.deleteDef');
    await press(target, 'Delete');
    await press(target, 'Confirm delete');
    expect(calls).toContain('properties.deleteDef');
  });

  it('move_up_saves_the_new_order', async () => {
    const { target, calls, api } = await mountSettings(Properties);
    const before = (await api.properties.listDefs()).map((d) => d.key);
    await setField(target, 'Property', before[1]);
    await press(target, 'Move up');
    expect(calls).toContain('properties.reorder');
    expect((await api.properties.listDefs()).map((d) => d.key)).toEqual([before[1], before[0], ...before.slice(2)]);
  });

  it('set_scope_conflict_shows_message_and_keeps_scope', async () => {
    const { target, api } = await mountSettings(Properties);
    const project = (await api.projects.list())[0].id;
    await setField(target, 'Property', 'area');
    await setField(target, 'Scope', String(project));
    await press(target, 'Set scope');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/outside the project/);
    expect((await api.properties.listDefs()).find((d) => d.key === 'area')?.project_id).toBeNull();
  });

  it('set_scope_to_all_projects_clears_it', async () => {
    const { target, calls } = await mountSettings(Properties);
    await setField(target, 'Property', 'area');
    await setField(target, 'Scope', '');
    await press(target, 'Set scope');
    expect(calls).toContain('properties.setScope');
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });
});
