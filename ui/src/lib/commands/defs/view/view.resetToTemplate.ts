import { defineCommand } from '../../registry';

defineCommand({
  id: 'view.resetToTemplate',
  label: 'Reset to template',
  icon: 'undo',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.resetToTemplate')),
});
