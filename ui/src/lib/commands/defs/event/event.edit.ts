import { defineCommand } from '../../registry';
import { emit, onEvent } from '../_event';

// The event page owns this control and listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'event.edit',
  label: 'Edit event',
  icon: 'edit',
  group: 'Event',
  noShortcutReason: 'Rare action, reached from the event page and the command menu.',
  when: onEvent,
  run: emit('event.edit'),
});
