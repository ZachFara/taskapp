import { describe, expect, it } from 'vitest';
import Backups from './Backups.svelte';
import { mountSettings, press, setField } from './testing';

describe('Backups', () => {
  it('snapshot now writes to the backup directory and shows the path', async () => {
    const { target, calls } = await mountSettings(Backups);
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    expect(calls).toContain('backup.snapshot');
    expect(target.textContent).toContain('backups-test/snapshots/mock-manual.db');
  });

  it('lists the backups found in the directory', async () => {
    const { target, api } = await mountSettings(Backups);
    await api.backup.snapshot('backups-test', 'older');
    await setField(target, 'Backup directory', 'backups-test');
    expect(target.textContent).toContain('backups-test/snapshots/mock-older.db');
  });

  it('shows the restore instruction instead of a restore button', async () => {
    const { target, calls } = await mountSettings(Backups);
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    expect(target.textContent).toContain(
      'To restore, quit the app and run: taskapp backup restore <file>',
    );
    expect(target.textContent).toContain('backups-test/snapshots/mock-manual.db');
    const buttons = [...target.querySelectorAll('button')].map((b) => b.textContent ?? '');
    expect(buttons.some((text) => /restore/i.test(text))).toBe(false);
    expect(calls).not.toContain('backup.restore');
  });
});
