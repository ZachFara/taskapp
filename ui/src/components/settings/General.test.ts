import { describe, expect, it } from 'vitest';
import General from './General.svelte';
import { field, mountSettings, press, setField } from './testing';

describe('General', () => {
  it('shows the saved issue prefix', async () => {
    const { target } = await mountSettings(General);
    expect(field(target, 'Issue prefix').value).toBe('ABC');
  });

  it('issue prefix rejects lowercase and empty values', async () => {
    const { target, calls } = await mountSettings(General);
    for (const bad of ['zf', '']) {
      await setField(target, 'Issue prefix', bad);
      await press(target, 'Save');
      expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/uppercase/i);
    }
    expect(calls).not.toContain('settings.set');
  });

  it('saves a valid prefix', async () => {
    const { target, api } = await mountSettings(General);
    await setField(target, 'Issue prefix', 'ABC');
    await press(target, 'Save');
    expect(await api.settings.get('issue_prefix')).toBe('ABC');
  });
});
