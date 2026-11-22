import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.toggle',
  label: 'Select row',
  icon: 'check',
  group: 'Table',
  shortcut: 'x',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.toggle')),
});
