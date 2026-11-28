import { askEmpty } from '../../../deleteIssues.svelte';
import { defineCommand } from '../../registry';

defineCommand({
  id: 'archive.empty', label: 'Empty archive…', icon: 'trash', group: 'Archive',
  noShortcutReason: 'it removes every archived issue for good, so it asks first and has no key',
  when: (ctx) => ctx.route === '/archive',
  run: () => askEmpty(),
});
