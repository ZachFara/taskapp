import { defineCommand } from '../../registry';
import { emit, onView } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'view.group.open',
  label: 'Group by…',
  icon: 'group',
  group: 'View',
  noShortcutReason: 'Reached from the view toolbar and the command menu.',
  when: onView,
  run: emit('view.group.open'),
});
