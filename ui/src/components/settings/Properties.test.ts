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
});
