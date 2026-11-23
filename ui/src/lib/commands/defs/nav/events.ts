import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.events',
  label: 'Go to Events',
  icon: 'calendar',
  group: 'Navigation',
  shortcut: 'g e',
  run: (ctx) => ctx.navigate('/events'),
});
