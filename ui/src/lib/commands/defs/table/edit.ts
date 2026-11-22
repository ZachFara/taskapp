import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.edit',
  menu: false,
  label: 'Edit cell',
  icon: 'edit',
  group: 'Table',
  scope: 'table',
  shortcut: 'enter',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.edit')),
});
