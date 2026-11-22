import { defineCommand } from '../../registry';

// The view and project pages own the panel's open state; the registry only decides when the key fires.
defineCommand({
  id: 'view.insights',
  label: 'Toggle insights',
  icon: 'target',
  group: 'View',
  shortcut: 'shift+i',
  when: (ctx) => ctx.route.startsWith('/view/') || ctx.route.startsWith('/project/'),
  run: () => dispatchEvent(new CustomEvent('command:insights.toggle')),
});
