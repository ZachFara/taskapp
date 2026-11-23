import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:view.new` and opens its form.
defineCommand({
  id: 'view.new', label: 'New view', icon: 'layers', group: 'Create', noShortcutReason: 'a button on the Views page and the command menu',
  run: (ctx) => openForm(ctx, '/views', 'command:view.new'),
});
