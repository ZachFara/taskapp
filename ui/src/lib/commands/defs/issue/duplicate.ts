import { defineCommand } from '../../registry';
import { refreshView, targets } from './_issue';

// One backend call for the whole selection, so undo removes every copy at once.
defineCommand({
  id: 'issue.duplicate', label: 'Duplicate', icon: 'copy', group: 'Issue', shortcut: 'mod+d',
  run: async (ctx) => {
    const ids = targets(ctx);
    if (ids.length) {
      await ctx.api.issues.duplicate(ids);
      refreshView();
    }
  },
});
