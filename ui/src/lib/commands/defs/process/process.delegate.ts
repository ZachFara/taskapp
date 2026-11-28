import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.delegate',
  label: 'Process: delegate (waiting on)',
  icon: 'user',
  group: 'Process Inbox',
  shortcut: 'y',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.delegate'),
});
