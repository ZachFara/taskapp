import { defineCommand } from '../../registry';

// ViewPage owns the layout; the registry only decides when the key fires.
defineCommand({
  id: 'view.layout.timeline',
  label: 'Gantt layout',
  icon: 'timeline',
  group: 'View',
  shortcut: 'v g',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.layout', { detail: 'Gantt' })),
});
