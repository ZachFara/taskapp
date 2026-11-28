import { defineCommand } from '../../registry';
import { refreshView, targets } from './_issue';

// Moves the due date of a repeating issue to its next occurrence without completing it.
defineCommand({
  id: 'issue.repeat.skip', label: 'Skip this occurrence', icon: 'cycle', group: 'Issue',
  noShortcutReason: 'Occasional action; reached from the Repeat row of the issue details and the command menu.',
  run: async (ctx) => {
    const [id] = targets(ctx);
    if (id === undefined) return;
    if ((await ctx.api.issues.get(id)).repeat === null) return;
    await ctx.api.issues.skipRepeat(id);
    refreshView();
  },
});
