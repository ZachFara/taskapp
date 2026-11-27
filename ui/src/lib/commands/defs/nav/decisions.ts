import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.decisions',
  label: 'Go to Decisions',
  icon: 'help',
  group: 'Navigation',
  shortcut: 'g q',
  run: (ctx) => ctx.navigate('/decisions'),
});
