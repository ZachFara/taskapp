import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.plan',
  label: 'Go to Today plan',
  icon: 'clock',
  group: 'Navigation',
  shortcut: 'g l',
  run: (ctx) => ctx.navigate('/plan'),
});
