import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.right',
  label: 'Next column',
  icon: 'chevron-right',
  group: 'Table',
  shortcut: ['l', 'arrowright'],
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.right')),
});
