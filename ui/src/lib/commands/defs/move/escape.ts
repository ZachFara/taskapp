import { defineCommand } from '../../registry';
import { selection } from '../../../selection.svelte';

// Closes the top overlay (the dialogs own Esc), else clears the selection. An open issue (panel or page) is closed by
// issue.detail.close, which also handles its editors first; two handlers on one Esc would close both. A top-level
// route does nothing: history.back() could leave the app.
defineCommand({
  id: 'move.escape',
  menu: false,
  label: 'Close or clear',
  icon: 'close',
  group: 'Navigation',
  shortcut: 'escape',
  run: (ctx) => {
    if (ctx.openIssueId !== null || ctx.route.startsWith('/issue/')) return;
    if (selection.selected.length) selection.clear();
  },
});
