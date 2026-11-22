import { defineCommand } from '../../registry';

defineCommand({
  id: 'app.reload',
  label: 'Reload',
  icon: 'cycle',
  group: 'Help',
  noShortcutReason: 'rare; the window menu and the command menu cover it',
  run: () => location.reload(),
});
