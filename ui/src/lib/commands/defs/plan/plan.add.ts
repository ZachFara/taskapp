import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The Today plan page listens for `command:plan.add` and opens its form.
defineCommand({
  id: 'plan.add', label: 'Time-block an issue', icon: 'clock', group: 'Create',
  noShortcutReason: 'a button on the Today plan page and the command menu',
  run: (ctx) => openForm(ctx, '/plan', 'command:plan.add'),
});
