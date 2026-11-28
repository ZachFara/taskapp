import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.goals',
  label: 'Go to Goals',
  icon: 'target',
  group: 'Navigation',
  shortcut: 'g k',
  run: (ctx) => ctx.navigate('/goals'),
});
