import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.cycles',
  label: 'Go to Cycles',
  icon: 'cycle',
  group: 'Navigation',
  shortcut: 'g c',
  run: (ctx) => ctx.navigate('/cycles'),
});
