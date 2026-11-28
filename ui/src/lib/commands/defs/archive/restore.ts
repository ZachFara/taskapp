import { defineCommand } from '../../registry';
import { patchOf, refreshView, targets } from '../issue/_issue';

defineCommand({
  id: 'archive.restore', label: 'Restore from archive', icon: 'undo', group: 'Archive',
  noShortcutReason: 'one click on the Archive page; a key here would be too easy to press by accident',
  when: (ctx) => ctx.route === '/archive' && targets(ctx).length > 0,
  run: async (ctx) => {
    await ctx.api.issues.bulkUpdate(targets(ctx), patchOf({}, { archive: false }));
    refreshView();
  },
});
