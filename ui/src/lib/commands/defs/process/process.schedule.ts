import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.schedule',
  label: 'Process: schedule (due on a day)',
  icon: 'calendar',
  group: 'Process Inbox',
  shortcut: 'v',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.schedule'),
});
