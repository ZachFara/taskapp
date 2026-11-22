import { defineCommand } from '../../registry';

defineCommand({
  id: 'view.pin',
  label: 'Pin or unpin view',
  icon: 'pin',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.pin')),
});
