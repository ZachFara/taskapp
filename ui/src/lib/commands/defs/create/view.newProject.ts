import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The Views page listens for `command:view.newProject` and opens the new-view form on projects.
defineCommand({
  id: 'view.newProject', label: 'New project view', icon: 'layers', group: 'Create', noShortcutReason: 'the Projects section menu in the sidebar and the command menu',
  run: (ctx) => openForm(ctx, '/views', 'command:view.newProject'),
});
