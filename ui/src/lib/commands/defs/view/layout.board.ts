import { defineCommand } from '../../registry';

// ViewPage owns the layout; the registry only decides when the key fires.
defineCommand({
  id: 'view.layout.board',
  label: 'Board layout',
  icon: 'board',
  group: 'View',
  shortcut: 'v b',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.layout', { detail: 'Board' })),
});
