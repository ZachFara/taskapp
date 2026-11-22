import { defineCommand } from '../../registry';

// The calendar view listens for the event; the registry only decides when the key fires.
defineCommand({
  id: 'calendar.prev',
  label: 'Previous week or month',
  icon: 'chevron-left',
  group: 'Calendar',
  shortcut: '[',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:calendar.prev')),
});
