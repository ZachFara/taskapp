import { defineCommand } from '../../registry';
import { keyOf, targets } from './_issue';

defineCommand({
  id: 'issue.copyKey', label: 'Copy key', icon: 'copy', group: 'Issue', shortcut: 'mod+c',
  run: async (ctx) => {
    if (targets(ctx).length) await navigator.clipboard.writeText(await keyOf(ctx));
  },
});
