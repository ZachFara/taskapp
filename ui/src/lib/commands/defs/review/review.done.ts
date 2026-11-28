import { defineCommand } from '../../registry';
import { toggleDone } from '../../../../components/review/review.svelte';

defineCommand({
  id: 'review.done',
  label: 'Weekly review: mark this step done',
  icon: 'check',
  group: 'Weekly review',
  noShortcutReason: 'reached from the Done button and the command menu',
  when: (ctx) => ctx.route === '/review',
  run: () => toggleDone(),
});
