import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.review',
  label: 'Go to Weekly review',
  icon: 'check',
  group: 'Navigation',
  shortcut: 'g r',
  run: (ctx) => ctx.navigate('/review'),
});
