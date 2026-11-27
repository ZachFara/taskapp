import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The event page owns this control and listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'event.link',
  label: 'Link an issue to this event',
  icon: 'link',
  group: 'Event',
  noShortcutReason: 'Reached from the event page and the command menu.',
  when: (ctx) => ctx.route.startsWith('/event/'),
  run: emit('event.link'),
});
