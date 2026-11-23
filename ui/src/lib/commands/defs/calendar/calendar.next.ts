import { defineCommand } from '../../registry';

// The calendar view listens for the event; the registry only decides when the key fires.
defineCommand({
  id: 'calendar.next',
  label: 'Next week or month',
  icon: 'chevron-right',
  group: 'Calendar',
  scope: 'calendar',
  shortcut: ']',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:calendar.next')),
});
