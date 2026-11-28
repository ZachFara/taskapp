import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The labels settings listen for `command:label.group.new` and focus the group name field.
defineCommand({
  id: 'label.group.new', label: 'New label group', icon: 'tag', group: 'Create', noShortcutReason: 'a button in Settings and the command menu',
  run: (ctx) => openForm(ctx, '/settings/labels', 'command:label.group.new'),
});
