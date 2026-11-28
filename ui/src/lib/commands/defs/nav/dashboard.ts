import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.dashboard',
  label: 'Go to Dashboard',
  icon: 'chart',
  group: 'Navigation',
  shortcut: 'g b',
  run: (ctx) => ctx.navigate('/dashboard'),
});
