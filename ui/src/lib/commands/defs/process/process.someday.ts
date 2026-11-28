import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.someday',
  label: 'Process: someday',
  icon: 'star',
  group: 'Process Inbox',
  shortcut: 'b',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.someday'),
});
