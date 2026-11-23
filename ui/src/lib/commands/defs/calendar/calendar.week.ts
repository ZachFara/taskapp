import { defineCommand } from '../../registry';

// The calendar view listens for the event; the registry only decides when the key fires.
defineCommand({
  id: 'calendar.week',
  label: 'Week view',
  icon: 'calendar',
  group: 'Calendar',
  scope: 'calendar',
  shortcut: 'w',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:calendar.week')),
});
