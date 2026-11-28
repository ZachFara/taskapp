import { askDelete } from '../../../deleteIssues.svelte';
import { defineCommand } from '../../registry';
import { targets } from './_issue';

// Menu and command menu only: it asks first, and a key for it would be too easy to press by accident.
defineCommand({
  id: 'issue.deleteForever', label: 'Delete permanently…', icon: 'trash', group: 'Issue',
  noShortcutReason: 'it removes the issue for good, so it asks first and has no key',
  when: (ctx) => ctx.route !== '/archive' && targets(ctx).length > 0,
  run: (ctx) => askDelete(targets(ctx)),
});
