import { defineCommand } from '../../registry';
import { refreshView } from '../../defs/issue/_issue';

defineCommand({
  id: 'focus.stop', label: 'Stop focus timer', icon: 'clock', group: 'Actions',
  noShortcutReason: 'Ends the running focus session from the command menu.',
  run: async (ctx) => {
    await ctx.api.focus.stop();
    refreshView();
  },
});
