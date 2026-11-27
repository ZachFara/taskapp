import { defineCommand } from '../../registry';
import { undoToast } from '../../../../components/undoState.svelte';

defineCommand({
  id: 'sync.now',
  label: 'Sync now',
  icon: 'cycle',
  group: 'Help',
  noShortcutReason: 'rare; the command menu covers it',
  run: async (ctx) => {
    const status = await ctx.api.sync.now();
    const failed = Object.values(status).find((s) => typeof s === 'object' && 'Error' in s);
    undoToast.value = failed
      ? { label: failed.Error.message, tone: 'error' }
      : { label: 'Synced', tone: 'success' };
  },
});
