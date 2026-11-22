import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.my',
  label: 'Go to My issues',
  icon: 'user',
  group: 'Navigation',
  shortcut: 'g m',
  run: (ctx) => ctx.navigate('/my'),
});
