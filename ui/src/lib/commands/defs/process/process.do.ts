import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.do',
  label: 'Process: do it now (Done)',
  icon: 'check',
  group: 'Process Inbox',
  shortcut: 'o',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.do'),
});
