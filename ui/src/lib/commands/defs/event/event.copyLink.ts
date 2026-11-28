import { defineCommand } from '../../registry';
import { copyLink } from '../../../deepLink';
import { emit, onEvent } from '../_event';

// On an event page the open event is the target; on the events list the page copies the event last right-clicked.
defineCommand({
  id: 'event.copyLink',
  label: 'Copy link to event',
  icon: 'link',
  group: 'Event',
  noShortcutReason: 'Rare action, reached from the event page, the right-click menu and the command menu.',
  when: onEvent,
  run: (ctx) => (ctx.route.startsWith('/event/') ? copyLink('event', ctx.route.split('/')[2]) : emit('event.copyLink')()),
});
