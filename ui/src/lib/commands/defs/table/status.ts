import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.status',
  label: 'Set status',
  icon: 'flag',
  group: 'Table',
  shortcut: 's',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.status')),
});
