import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Goals pages own this action and listen for the event; the registry only decides when it is offered.
defineCommand({
  id: 'goal.edit',
  label: 'Edit goal',
  icon: 'edit',
  group: 'Goal',
  noShortcutReason: 'Rare action, reached from the goal page and the command menu.',
  when: (ctx) => ctx.route.startsWith('/goal/'),
  run: emit('goal.edit'),
});
