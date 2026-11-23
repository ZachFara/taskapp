import { defineCommand } from '../../registry';

// Sections are reached from the Settings page; the key for the page itself is g s.
defineCommand({
  id: 'settings.labels',
  label: 'Labels',
  icon: 'tag',
  group: 'Settings',
  noShortcutReason: 'Reached from the settings list or the command menu',
  run: (ctx) => ctx.navigate('/settings/labels'),
});
