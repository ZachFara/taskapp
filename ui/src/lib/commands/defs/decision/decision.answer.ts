import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Decisions page owns this: it answers the decision last clicked or focused there with its recommendation.
defineCommand({
  id: 'decision.answer',
  label: 'Answer decision with the recommendation',
  icon: 'check',
  group: 'Decision',
  noShortcutReason: 'Reached from the Decisions page (the card buttons and the command menu).',
  when: (ctx) => ctx.route === '/decisions',
  run: emit('decision.answer'),
});
