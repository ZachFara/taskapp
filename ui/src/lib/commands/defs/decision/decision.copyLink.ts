import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Decisions page owns this: it copies the link of the decision last clicked or focused there.
defineCommand({
  id: 'decision.copyLink',
  label: 'Copy link to decision',
  icon: 'link',
  group: 'Decision',
  noShortcutReason: 'Reached from the Decisions page and the command menu.',
  when: (ctx) => ctx.route === '/decisions',
  run: emit('decision.copyLink'),
});
