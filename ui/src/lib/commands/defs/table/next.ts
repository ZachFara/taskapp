import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.next',
  label: 'Next row',
  icon: 'chevron-down',
  group: 'Table',
  shortcut: ['j', 'arrowdown'],
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.next')),
});
