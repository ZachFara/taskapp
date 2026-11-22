import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.pinned',
  label: 'Go to Pinned',
  icon: 'pin',
  group: 'Navigation',
  shortcut: 'g h',
  run: (ctx) => ctx.navigate('/pinned'),
});
