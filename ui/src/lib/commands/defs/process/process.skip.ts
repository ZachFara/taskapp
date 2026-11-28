import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.skip',
  label: 'Process: skip to the next issue',
  icon: 'chevron-right',
  group: 'Process Inbox',
  shortcut: 'shift+arrowright',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.skip'),
});
