import { defineCommand } from '../../registry';

// ViewPage owns the layout; the registry only decides when the key fires.
defineCommand({
  id: 'view.layout.list',
  label: 'List layout',
  icon: 'list',
  group: 'View',
  shortcut: '1',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.layout', { detail: 'List' })),
});
