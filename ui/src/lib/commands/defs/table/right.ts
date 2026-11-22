import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.right',
  menu: false,
  label: 'Cell to the right',
  icon: 'chevron-right',
  group: 'Table',
  scope: 'table',
  shortcut: 'arrowright',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.right')),
});
