import { describe, expect, it } from 'vitest';
import Export from './Export.svelte';
import { field, mountSettings, press, setField } from './testing';

describe('Export', () => {
  it('export_toggle_off_by_default', async () => {
    const { target } = await mountSettings(Export);
    expect(field(target, 'Enable export').checked).toBe(false);
  });

  it('export directory rejects an empty path when export is enabled', async () => {
    const { target, calls } = await mountSettings(Export);
    await setField(target, 'Enable export', true);
    await setField(target, 'Export directory', '  ');
    await press(target, 'Save');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/directory/i);
    expect(calls).not.toContain('settings.set');
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
});
