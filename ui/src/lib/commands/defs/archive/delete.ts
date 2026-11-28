import { askDelete } from '../../../deleteIssues.svelte';
import { defineCommand } from '../../registry';
import { targets } from '../issue/_issue';

defineCommand({
  id: 'archive.delete', label: 'Delete selected permanently…', icon: 'trash', group: 'Archive',
  noShortcutReason: 'it removes issues for good, so it asks first and has no key',
  when: (ctx) => ctx.route === '/archive' && targets(ctx).length > 0,
  run: (ctx) => askDelete(targets(ctx)),
});
