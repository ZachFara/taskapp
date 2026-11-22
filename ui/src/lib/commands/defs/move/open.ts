import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.open',
  label: 'Open issue',
  icon: 'panel',
  group: 'Navigation',
  shortcut: 'enter',
  run: async (ctx) => {
    const id = selection.focused;
    if (id === null) return;
    const issue = await ctx.api.issues.get(id);
    ctx.navigate(`/issue/${issue.key}`);
  },
});
