import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The Events page listens for `command:event.new` and opens its form.
defineCommand({
  id: 'event.new', label: 'New event', icon: 'calendar', group: 'Create', noShortcutReason: 'a button on the Events page and the command menu',
  run: (ctx) => openForm(ctx, '/events', 'command:event.new'),
});
