import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The events list owns this action: it opens the event last right-clicked there, or the one it has picked.
defineCommand({
  id: 'event.open',
  label: 'Open event',
  icon: 'calendar',
  group: 'Event',
  noShortcutReason: 'Reached from the events list (right-click and the command menu).',
  when: (ctx) => ctx.route === '/events',
  run: emit('event.open'),
});
