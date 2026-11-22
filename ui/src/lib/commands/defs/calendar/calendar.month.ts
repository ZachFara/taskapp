import { defineCommand } from '../../registry';

// The calendar view listens for the event; the registry only decides when the key fires.
defineCommand({
  id: 'calendar.month',
  label: 'Month view',
  icon: 'calendar',
  group: 'Calendar',
  scope: 'calendar',
  shortcut: 'm',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:calendar.month')),
});
