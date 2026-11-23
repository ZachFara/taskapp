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

  it.each([[null], [undefined], [''], ['Etc/Unlisted']])('shows a zone for a stored time zone of %j', async (stored) => {
    const { target } = await mountSettings(General, {
      settings: { get: async (key: unknown) => (key === 'timezone' ? stored : null) },
    });
    const shown = field(target, 'Time zone').textContent ?? '';
    expect(shown).not.toMatch(/^\s*Select\s*$/);
    expect(shown).toContain(stored ? stored : '(system)');
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

  it('picking a look applies it at once', async () => {
    const { target } = await mountSettings(General);
    await press(target, 'Jira');
    expect(document.documentElement.dataset.skin).toBe('jira');
    expect(target.querySelector('button[aria-pressed="true"]')?.textContent).toContain('Jira');
    await press(target, 'Survey map');
    expect(document.documentElement.hasAttribute('data-skin')).toBe(false);
  });

  it('blank fields say what goes in them, and a time zone change waits for a restart', async () => {
    const { target } = await mountSettings(General);
    expect(field(target, 'Workspace name').placeholder).not.toBe('');
    expect(target.textContent).toContain('applies after restart');
  });

  it('the settings.save command saves this page', async () => {
    const { target, api } = await mountSettings(General);
    await setField(target, 'Workspace key', 'QZ');
    window.dispatchEvent(new CustomEvent('command:settings.save'));
    await new Promise((r) => setTimeout(r, 0));
    expect(await api.settings.get('issue_prefix')).toBe('QZ');
  });
});
