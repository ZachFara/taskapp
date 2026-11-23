import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:label.new` and opens its form.
defineCommand({
  id: 'label.new', label: 'New label', icon: 'tag', group: 'Create', noShortcutReason: 'a button in Settings and the command menu',
  run: (ctx) => openForm(ctx, '/settings/labels', 'command:label.new'),
});
