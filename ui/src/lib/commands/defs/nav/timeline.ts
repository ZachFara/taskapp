import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.timeline',
  label: 'Go to Timeline',
  icon: 'timeline',
  group: 'Navigation',
  shortcut: 'g t',
  run: (ctx) => ctx.navigate('/projects/timeline'),
});
