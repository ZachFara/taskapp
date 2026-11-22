import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:template.new` and opens its form.
defineCommand({
  id: 'template.new', label: 'New template', icon: 'copy', group: 'Create', noShortcutReason: 'a button in Settings and the command menu',
  run: (ctx) => openForm(ctx, '/settings/templates', 'command:template.new'),
});
