import { defineCommand } from '../../registry';
import { refreshView } from '../issue/_issue';
import { runUndo, undoToast } from '../../../../components/undoState.svelte';

// Not global: inside a text field mod+z stays the field's own undo.
defineCommand({
  id: 'history.undo',
  label: 'Undo',
  icon: 'undo',
  group: 'History',
  shortcut: ['mod+z', 'u'],
  run: async (ctx) => {
    if ((await ctx.api.history.undoStatus()).undo) return runUndo(ctx.api).then(refreshView);
    undoToast.value = { label: 'Nothing to undo', tone: 'info' };
    setTimeout(() => undoToast.value?.label === 'Nothing to undo' && (undoToast.value = null), 3_000);
  },
});
