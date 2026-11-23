import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.views',
  label: 'Go to Views',
  icon: 'layers',
  group: 'Navigation',
  shortcut: 'g v',
  run: (ctx) => ctx.navigate('/views'),
});
