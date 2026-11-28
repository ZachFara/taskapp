import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Goals pages own this action and listen for the event; the registry only decides when it is offered.
defineCommand({
  id: 'goal.delete',
  label: 'Delete goal',
  icon: 'trash',
  group: 'Goal',
  noShortcutReason: 'Destructive; reached from the goal page and the command menu, and undo brings it back.',
  when: (ctx) => ctx.route.startsWith('/goal/'),
  run: emit('goal.delete'),
});
