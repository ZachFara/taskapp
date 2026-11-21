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

  it('export now calls sync', async () => {
    const { target, calls } = await mountSettings(Export);
    await press(target, 'Export now');
    expect(calls).toContain('sync.now');
  });
});
