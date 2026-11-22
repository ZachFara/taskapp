import { defineCommand } from '../../registry';
import { targets } from './_issue';

// Pins are not part of the backend's undo history, so there is no step to make single; each issue toggles on its own.
defineCommand({
  id: 'issue.pin', label: 'Pin or unpin', icon: 'pin', group: 'Issue', shortcut: '.',
  run: async (ctx) => {
    const pins = (await ctx.api.pins.listResolved()).map((r) => r.pin);
    for (const id of targets(ctx)) {
      const { key } = await ctx.api.issues.get(id);
      const had = pins.find((p) => p.kind === 'Issue' && p.target === key);
      if (had) await ctx.api.pins.unpin(had.id);
      else await ctx.api.pins.pin('Issue', key, null);
    }
  },
});
