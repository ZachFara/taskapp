import { defineCommand } from '../../registry';
import { back } from '../../../../components/review/review.svelte';

defineCommand({
  id: 'review.back',
  label: 'Weekly review: previous step',
  icon: 'chevron-left',
  group: 'Weekly review',
  noShortcutReason: 'reached from the Back button and the command menu',
  when: (ctx) => ctx.route === '/review',
  run: () => back(),
});
