import { defineCommand } from '../../registry';
import { emit } from '../_event';

// Same target as decision.answer: the decision last clicked or focused on the page.
defineCommand({
  id: 'decision.withdraw',
  label: 'Withdraw decision',
  icon: 'close',
  group: 'Decision',
  noShortcutReason: 'Reached from the Decisions page (the card buttons and the command menu).',
  when: (ctx) => ctx.route === '/decisions',
  run: emit('decision.withdraw'),
});
