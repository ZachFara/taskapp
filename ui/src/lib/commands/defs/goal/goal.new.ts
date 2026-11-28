import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Goals pages own this action and listen for the event; the registry only decides when it is offered.
defineCommand({
  id: 'goal.new',
  label: 'New goal',
  icon: 'plus',
  group: 'Goal',
  noShortcutReason: 'Reached from the Goals page and the command menu.',
  when: (ctx) => ctx.route === '/goals',
  run: emit('goal.new'),
});
