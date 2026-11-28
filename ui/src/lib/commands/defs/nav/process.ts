import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.process',
  label: 'Process Inbox',
  icon: 'inbox',
  group: 'Navigation',
  shortcut: 'g o',
  run: (ctx) => ctx.navigate('/process'),
});
