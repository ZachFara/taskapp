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

  it('restore passes the chosen file and shows the refusal', async () => {
    const { target, calls } = await mountSettings(Backups);
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    await press(target, 'Restore');
    expect(calls).toContain('backup.restore');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/open elsewhere/);
  });

  it('restore success is reported', async () => {
    const { target } = await mountSettings(Backups, { backup: { restore: async () => null } });
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    await press(target, 'Restore');
    expect(target.querySelector('[role="alert"]')).toBeNull();
    expect(target.textContent).toMatch(/restored/i);
  });
});
