import { defineCommand } from '../../registry';

// ViewPage owns the layout; the registry only decides when the key fires.
defineCommand({
  id: 'view.layout.calendar',
  label: 'Calendar layout',
  icon: 'calendar',
  group: 'View',
  shortcut: '5',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.layout', { detail: 'Calendar' })),
});
