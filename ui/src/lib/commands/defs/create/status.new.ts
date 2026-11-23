import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:status.new` and opens its form.
defineCommand({
  id: 'status.new', label: 'New status', icon: 'circle', group: 'Create', noShortcutReason: 'a button in Settings and the command menu',
  run: (ctx) => openForm(ctx, '/settings/statuses', 'command:status.new'),
});
