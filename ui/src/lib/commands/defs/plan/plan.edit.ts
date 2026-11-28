import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The page owns the selected block and listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'plan.edit', label: 'Change time block start and length', icon: 'edit', group: 'Plan',
  noShortcutReason: 'Rare action; blocks also move and resize by dragging.',
  when: (ctx) => ctx.route === '/plan',
  run: emit('plan.edit'),
});
