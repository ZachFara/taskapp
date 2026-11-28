import { defineCommand } from '../../registry';
import { start } from '../../../../components/review/review.svelte';

defineCommand({
  id: 'review.start',
  label: 'Weekly review: start',
  icon: 'check',
  group: 'Weekly review',
  noShortcutReason: 'reached from the command menu and the sidebar',
  run: (ctx) => (start(), ctx.navigate('/review')),
});
