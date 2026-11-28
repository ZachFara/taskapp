import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.waiting',
  label: 'Go to Waiting for',
  icon: 'clock',
  group: 'Navigation',
  shortcut: 'g w',
  run: (ctx) => ctx.navigate('/waiting'),
});
