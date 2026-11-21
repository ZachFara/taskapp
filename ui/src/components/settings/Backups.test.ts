import { describe, expect, it } from 'vitest';
import Backups from './Backups.svelte';
import { hasButton, mountSettings, press, setField } from './testing';

describe('Backups', () => {
  it('snapshot now writes to the backup directory and shows the path', async () => {
    const { target, calls } = await mountSettings(Backups);
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    expect(calls).toContain('backup.snapshot');
    expect(target.textContent).toContain('backups-test/mock-manual.db');
  });

  it('restore is shown disabled because no restore command exists yet', async () => {
    const { target } = await mountSettings(Backups);
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    expect(hasButton(target, 'Restore')).toBe(true);
    expect([...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Restore')?.disabled).toBe(true);
  });
});
