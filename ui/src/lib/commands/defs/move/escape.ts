import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

// Closes the panel first, then clears the selection, then steps back out of the current route.
defineCommand({
  id: 'move.escape',
  label: 'Close or clear',
  icon: 'close',
  group: 'Navigation',
  shortcut: 'escape',
  run: (ctx) => {
    if (ctx.openIssueId !== null) return history.back();
    if (selection.selected.length) return selection.clear();
    if (ctx.route !== '/') history.back();
  },
});
