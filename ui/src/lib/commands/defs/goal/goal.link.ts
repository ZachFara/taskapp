import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Goals pages own this action and listen for the event; the registry only decides when it is offered.
defineCommand({
  id: 'goal.link',
  label: 'Link a project to this goal',
  icon: 'link',
  group: 'Goal',
  noShortcutReason: 'Reached from the goal page and the command menu.',
  when: (ctx) => ctx.route.startsWith('/goal/'),
  run: emit('goal.link'),
});
