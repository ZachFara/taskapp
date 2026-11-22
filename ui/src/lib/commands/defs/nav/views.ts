import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.views',
  label: 'Go to Views',
  icon: 'board',
  group: 'Navigation',
  shortcut: 'g v',
  run: (ctx) => ctx.navigate('/views'),
});
