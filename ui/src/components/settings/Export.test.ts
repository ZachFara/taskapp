import { describe, expect, it } from 'vitest';
import Export from './Export.svelte';
import { field, mountSettings, press, setField, toggle } from './testing';

describe('Export', () => {
  it('export_toggle_off_by_default', async () => {
    const { target } = await mountSettings(Export);
    expect(field(target, 'Enable export').getAttribute('aria-checked')).toBe('false');
  });

  it('export directory rejects an empty path when export is enabled', async () => {
    const { target, calls } = await mountSettings(Export);
    await toggle(target, 'Enable export');
    await setField(target, 'Export directory', '  ');
    await press(target, 'Save');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/directory/i);
    expect(calls).not.toContain('settings.set');
  });

  it('saves the export settings with the same keys as before', async () => {
    const { target, api } = await mountSettings(Export);
    await toggle(target, 'Enable export');
    await setField(target, 'Export directory', '/exports/cairn');
    await setField(target, 'Export remote', 'git@example.test:notes.git');
    await press(target, 'Save');
    expect(await api.settings.get('export_enabled')).toBe(true);
    expect(await api.settings.get('export_dir')).toBe('/exports/cairn');
    expect(await api.settings.get('export_remote')).toBe('git@example.test:notes.git');
  });

  it('export now calls the export command and reports it', async () => {
    const { target, calls } = await mountSettings(Export);
    await press(target, 'Export now');
    expect(calls).toContain('export.now');
    expect(target.textContent).toMatch(/exported/i);
  });

  it('export now shows the error when it fails', async () => {
    const fail = async () => {
      throw { kind: 'Invalid', message: 'choose an export directory first' };
    };
    const { target } = await mountSettings(Export, { export: { now: fail } });
    await press(target, 'Export now');
    expect(target.querySelector('[role="alert"]')?.textContent).toContain('export directory');
  });

  it('blank export fields show a placeholder', async () => {
    const { target } = await mountSettings(Export);
    expect(field(target, 'Export directory').placeholder).not.toBe('');
    expect(field(target, 'Export remote').placeholder).not.toBe('');
  });

  it('shows the remote-ahead message with a Pull and apply button that reports conflicts', async () => {
    const status = async () => ({ reminders: 'Off', gcal: 'Off', backup: 'Off', export: { Error: { at: 't', message: 'The remote has changes from another machine.' } } });
    const pull = async () => ({ created: 2, warnings: ['ABC-2: changed on both machines; kept this machine\'s version'] });
    const { target, calls } = await mountSettings(Export, { sync: { status }, export: { pull } });
    await setField(target, 'Export remote', 'git@example.test:notes.git');
    expect(target.querySelector('[role="alert"]')?.textContent).toContain('another machine');
    await press(target, 'Pull and apply');
    expect(calls).toContain('export.pull');
    expect(target.querySelector('[data-last-sync]')?.textContent).toMatch(/2 issues changed, 1 conflict/);
    expect(target.textContent).toContain('ABC-2');
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });
});
