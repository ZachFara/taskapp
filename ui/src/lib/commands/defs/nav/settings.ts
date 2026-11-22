import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.settings',
  label: 'Go to Settings',
  icon: 'settings',
  group: 'Navigation',
  shortcut: 'g s',
  run: (ctx) => ctx.navigate('/settings/general'),
});
