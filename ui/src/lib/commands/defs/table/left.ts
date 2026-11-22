import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.left',
  label: 'Previous column',
  icon: 'chevron-right',
  group: 'Table',
  shortcut: ['h', 'arrowleft'],
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.left')),
});
