import { defineCommand } from '../../registry';
import { emit } from '../_event';

// The Process Inbox page owns this: it acts on the issue it is showing.
defineCommand({
  id: 'process.defer',
  label: 'Process: defer (hide until a day)',
  icon: 'clock',
  group: 'Process Inbox',
  shortcut: 'z',
  when: (ctx) => ctx.route === '/process',
  run: emit('process.defer'),
});
