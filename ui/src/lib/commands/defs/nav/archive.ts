import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.archive',
  label: 'Go to Archive',
  icon: 'archive',
  group: 'Navigation',
  shortcut: 'g a',
  run: (ctx) => ctx.navigate('/archive'),
});
