import { defineCommand } from '../../registry';
import { next } from '../../../../components/review/review.svelte';

defineCommand({
  id: 'review.next',
  label: 'Weekly review: next step',
  icon: 'chevron-right',
  group: 'Weekly review',
  noShortcutReason: 'reached from the Next button and the command menu',
  when: (ctx) => ctx.route === '/review',
  run: () => next(),
});
