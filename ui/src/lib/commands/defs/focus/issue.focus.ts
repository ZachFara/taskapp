import { defineCommand } from '../../registry';
import { navigate } from '../../../../app/routes';
import { targets } from '../../defs/issue/_issue';

defineCommand({
  id: 'issue.focus', label: 'Focus on this issue', icon: 'target', group: 'Issue',
  noShortcutReason: 'Command menu, right-click and the issue header button.',
  run: async (ctx) => {
    const [id] = targets(ctx);
    if (id === undefined) return;
    navigate(`/focus/${(await ctx.api.issues.get(id)).key}`);
  },
});
