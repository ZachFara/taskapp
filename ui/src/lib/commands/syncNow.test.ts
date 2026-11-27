import { describe, expect, it } from 'vitest';
import { undoToast } from '../../components/undoState.svelte';
import { listCommands, type Ctx } from './index';

const run = async (status: object) => {
  const ctx = { route: '/', selection: [], focusedIssueId: null, openIssueId: null, navigate: () => {}, api: { sync: { now: async () => status } } } as unknown as Ctx;
  await listCommands(ctx, { all: true }).find((c) => c.id === 'sync.now')!.run(ctx);
  return undoToast.value;
};

describe('sync.now', () => {
  it('is in the command menu and shows the helper message when a step failed', async () => {
    const at = '2026-11-27T00:00:00Z';
    expect(await run({ reminders: { Ok: { at } }, gcal: 'Off' })).toMatchObject({ label: 'Synced', tone: 'success' });
    const msg = 'Reminders access is off: press Authorize in Settings > Reminders';
    expect(await run({ reminders: { Error: { at, message: msg } }, gcal: 'Off' })).toMatchObject({ label: msg, tone: 'error' });
  });
});
