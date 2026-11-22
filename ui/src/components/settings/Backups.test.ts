import { describe, expect, it } from 'vitest';
import Backups from './Backups.svelte';
import { mountSettings, press, setField } from './testing';

// A small store standing in for the disk: snapshots written here are the ones listed back.
function fakeBackups() {
  const store: string[] = [];
  return {
    snapshot: async (dir: unknown, reason: unknown) => {
      const path = `${dir}/snapshots/${reason}.db`;
      store.push(path);
      return path;
    },
    list: async (dir: unknown) => store.filter((p) => p.startsWith(`${dir}/`)),
  };
}

describe('Backups', () => {
  it('snapshot now writes to the backup directory and shows the path', async () => {
    const { target, calls } = await mountSettings(Backups, { backup: fakeBackups() });
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    expect(calls).toContain('backup.snapshot');
    expect(target.textContent).toContain('backups-test/snapshots/manual.db');
  });

  it('lists the backups found in the directory', async () => {
    const { target, api } = await mountSettings(Backups, { backup: fakeBackups() });
    const path = await api.backup.snapshot('backups-test', 'older');
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Save folder');
    expect(target.textContent).toContain(path);
  });

  it('shows the restore instruction instead of a restore button', async () => {
    const { target, calls } = await mountSettings(Backups, { backup: fakeBackups() });
    await setField(target, 'Backup directory', 'backups-test');
    await press(target, 'Snapshot now');
    expect(target.textContent).toContain(
      'To restore, quit the app and run: taskapp backup restore <file>',
    );
    const buttons = [...target.querySelectorAll('button')].map((b) => b.textContent ?? '');
    expect(buttons.some((text) => /restore/i.test(text))).toBe(false);
    expect(calls).not.toContain('backup.restore');
  });
});
