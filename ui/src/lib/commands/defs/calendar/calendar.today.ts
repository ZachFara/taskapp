import { defineCommand } from '../../registry';

// The calendar view listens for the event; the registry only decides when the key fires.
defineCommand({
  id: 'calendar.today',
  label: 'Go to today',
  icon: 'calendar',
  group: 'Calendar',
  shortcut: 't',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:calendar.today')),
});
