import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'plan.remove', label: 'Remove time block', icon: 'trash', group: 'Plan',
  noShortcutReason: 'Reached from the block and the command menu; undo brings it back.',
  when: (ctx) => ctx.route === '/plan',
  run: emit('plan.remove'),
});
