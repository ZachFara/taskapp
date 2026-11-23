import { defineCommand } from '../../registry';

// The focused chart listens for the event; the registry only decides when the key fires.
defineCommand({
  id: 'chart.day.prev',
  menu: false,
  label: 'Previous day on the chart',
  icon: 'chevron-right',
  group: 'View',
  shortcut: 'arrowleft',
  when: () => !!document.activeElement?.closest('[data-chart]'),
  run: () => dispatchEvent(new CustomEvent('command:chart.day', { detail: -1 })),
});
