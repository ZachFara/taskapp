import { defineCommand } from '../../registry';

defineCommand({
  id: 'settings.project_statuses',
  label: 'Project statuses',
  icon: 'circle',
  group: 'Settings',
  noShortcutReason: 'Reached from the settings list or the command menu',
  run: (ctx) => ctx.navigate('/settings/project-statuses'),
});
