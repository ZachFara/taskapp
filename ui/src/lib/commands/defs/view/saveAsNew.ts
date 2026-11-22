import { defineCommand } from '../../registry';

defineCommand({
  id: 'view.saveAsNew',
  label: 'Save as new view',
  icon: 'copy',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.saveAsNew')),
});
