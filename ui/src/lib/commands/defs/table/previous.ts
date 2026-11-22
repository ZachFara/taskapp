import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.previous',
  label: 'Previous row',
  icon: 'chevron-up',
  group: 'Table',
  shortcut: ['k', 'arrowup'],
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.previous')),
});
