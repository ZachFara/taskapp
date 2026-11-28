import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.back',
  label: 'Process: back to the previous issue',
  icon: 'chevron-left',
  group: 'Process Inbox',
  shortcut: 'shift+arrowleft',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.back'),
});
