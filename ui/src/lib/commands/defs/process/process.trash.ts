import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.trash',
  label: 'Process: trash (archive)',
  icon: 'trash',
  group: 'Process Inbox',
  shortcut: 'q',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.trash'),
});
