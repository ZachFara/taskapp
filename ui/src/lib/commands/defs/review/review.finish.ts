import { defineCommand } from '../../registry';
import { allDone, finish } from '../../../../components/review/review.svelte';

defineCommand({
  id: 'review.finish',
  label: 'Weekly review: finish and record the date',
  icon: 'check',
  group: 'Weekly review',
  noShortcutReason: 'reached from the Finish button and the command menu',
  when: (ctx) => ctx.route === '/review' && allDone(),
  run: () => finish(),
});
