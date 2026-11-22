import { defineCommand } from '../../registry';
import { keyOf, targets } from './_issue';

defineCommand({
  id: 'issue.copyLink', label: 'Copy link', icon: 'link', group: 'Issue', shortcut: 'mod+shift+c',
  run: async (ctx) => {
    if (targets(ctx).length) await navigator.clipboard.writeText(`taskapp://issue/${await keyOf(ctx)}`);
  },
});
