import { defineCommand } from '../../registry';
import { targets } from './_issue';

defineCommand({
  id: 'issue.pin', label: 'Pin or unpin', icon: 'pin', group: 'Issue', shortcut: '.',
  run: async (ctx) => {
    const pins = await ctx.api.pins.list();
    for (const id of targets(ctx)) {
      const { key } = await ctx.api.issues.get(id);
      const had = pins.find((p) => p.kind === 'Issue' && p.target === key);
      if (had) await ctx.api.pins.unpin(had.id);
      else await ctx.api.pins.pin('Issue', key, null);
    }
  },
});
