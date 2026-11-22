import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

// Clears the selection, then steps back out of the current route. An open issue (panel or page) is closed by
// issue.detail.close, which also handles its editors first; two handlers on one Esc would close both.
defineCommand({
  id: 'move.escape',
  label: 'Close or clear',
  icon: 'close',
  group: 'Navigation',
  shortcut: 'escape',
  run: (ctx) => {
    if (ctx.openIssueId !== null || ctx.route.startsWith('/issue/')) return;
    if (selection.selected.length) return selection.clear();
    if (ctx.route !== '/') history.back();
  },
});
