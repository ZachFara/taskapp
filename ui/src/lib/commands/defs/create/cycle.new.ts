import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:cycle.new` and opens its form.
defineCommand({
  id: 'cycle.new', label: 'New cycle', icon: 'cycle', group: 'Create', noShortcutReason: 'a button on the Cycles page and the command menu',
  run: (ctx) => openForm(ctx, '/cycles', 'command:cycle.new'),
});
