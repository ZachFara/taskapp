import { defineCommand } from '../../registry';

defineCommand({
  id: 'settings.open',
  label: 'Settings',
  icon: 'settings',
  group: 'Navigate',
  shortcut: 'g s',
  run: (ctx) => ctx.navigate('/settings/general'),
});
