import { describe, expect, it } from 'vitest';
import General from './General.svelte';
import { field, mountSettings, pick, press, setField, statusText } from './testing';

describe('General', () => {
  it('shows the stored workspace key', async () => {
    const { target } = await mountSettings(General, {
      settings: { get: async (key: unknown) => (key === 'issue_prefix' ? 'QX' : null) },
    });
    expect(field(target, 'Workspace key').value).toBe('QX');
  });

  it('workspace key rejects anything but 2 to 5 uppercase letters', async () => {
    const { target, calls } = await mountSettings(General);
    for (const bad of ['A', 'ABCDEF', 'A1', 'zf', '']) {
      await setField(target, 'Workspace key', bad);
      await press(target, 'Save');
      expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/2 to 5 uppercase letters/);
    }
    expect(calls).not.toContain('settings.set');
  });

  it('saves a valid workspace key and says so', async () => {
    const { target, api } = await mountSettings(General);
    await setField(target, 'Workspace key', 'QX');
    await press(target, 'Save');
    expect(await api.settings.get('issue_prefix')).toBe('QX');
    expect(statusText(target)).toMatch(/saved/i);
  });

  it('shows the key the next issue will get, and that existing keys stay', async () => {
    const { target } = await mountSettings(General);
    await setField(target, 'Workspace key', 'QX');
    expect(target.textContent).toContain('QX-1');
    expect(target.textContent).toMatch(/existing issue keys (do not|keep working)/i);
  });

  it('saves the time zone, theme and week start with the same keys as before', async () => {
    const { target, api } = await mountSettings(General);
    await pick(target, 'Time zone', 'Europe/Paris');
    await pick(target, 'Week starts on', 'Monday');
    await pick(target, 'Theme', 'Dark');
    await press(target, 'Save');
    expect(await api.settings.get('timezone')).toBe('Europe/Paris');
    expect(await api.settings.get('week_start')).toBe('monday');
    expect(await api.settings.get('theme')).toBe('dark');
  // Opening the time zone list renders every zone; under load that takes longer than the default 5s.
  }, 30_000);
});
