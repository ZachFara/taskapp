import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.tickler',
  label: 'Go to Tickler',
  icon: 'clock',
  group: 'Navigation',
  shortcut: 'g f',
  run: (ctx) => ctx.navigate('/tickler'),
});
