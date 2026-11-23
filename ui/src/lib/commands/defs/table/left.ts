import { defineCommand } from '../../registry';

// A focused chart owns the arrows (chart.day.*); the table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.left',
  menu: false,
  label: 'Cell to the left',
  icon: 'chevron-right',
  group: 'Table',
  scope: 'table',
  shortcut: 'arrowleft',
  when: (ctx) => ctx.route.startsWith('/view/') && !document.activeElement?.closest('[data-chart]'),
  run: () => dispatchEvent(new CustomEvent('command:table.left')),
});
