import { defineCommand } from '../../registry';

// Sections are reached from the Settings page; the key for the page itself is g s.
defineCommand({
  id: 'settings.export',
  label: 'Export',
  icon: 'download',
  group: 'Settings',
  noShortcutReason: 'Reached from the settings list or the command menu',
  run: (ctx) => ctx.navigate('/settings/export'),
});
