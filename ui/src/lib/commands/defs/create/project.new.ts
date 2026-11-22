import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:project.new` and opens its form.
defineCommand({
  id: 'project.new', label: 'New project', icon: 'project', group: 'Create', noShortcutReason: 'a button on the Projects page and the command menu',
  run: (ctx) => openForm(ctx, '/projects', 'command:project.new'),
});
