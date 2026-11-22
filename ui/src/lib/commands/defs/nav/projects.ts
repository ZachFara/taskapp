import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.projects',
  label: 'Go to Projects',
  icon: 'folder',
  group: 'Navigation',
  shortcut: 'g p',
  run: (ctx) => ctx.navigate('/projects'),
});
