import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.inbox',
  label: 'Go to Inbox',
  icon: 'inbox',
  group: 'Navigation',
  shortcut: 'g i',
  run: (ctx) => ctx.navigate('/inbox'),
});
