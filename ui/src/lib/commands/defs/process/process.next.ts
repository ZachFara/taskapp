import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.next',
  label: 'Process: next action (pick a project)',
  icon: 'arrow-right',
  group: 'Process Inbox',
  shortcut: 'n',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.next'),
});
