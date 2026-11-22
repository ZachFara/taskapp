import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.priority',
  label: 'Set priority',
  icon: 'flag',
  group: 'Table',
  shortcut: 'p',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.priority')),
});
