import { defineCommand } from '../../registry';

defineCommand({
  id: 'view.saveAsTemplate',
  label: 'Save as template',
  icon: 'bookmark',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.saveAsTemplate')),
});
