import { defineCommand } from '../../registry';

defineCommand({
  id: 'view.delete',
  label: 'Delete view',
  icon: 'trash',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.delete')),
});
