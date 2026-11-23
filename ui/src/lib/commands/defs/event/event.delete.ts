import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The event page owns this control and listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'event.delete',
  label: 'Delete event',
  icon: 'trash',
  group: 'Event',
  noShortcutReason: 'Destructive; reached from the event page and the command menu, and undo brings it back.',
  when: (ctx) => ctx.route.startsWith('/event/'),
  run: emit('event.delete'),
});
