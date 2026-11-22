import { defineCommand } from '../../registry';
import { emit, onView } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'view.duplicate',
  label: 'Save as new view',
  icon: 'copy',
  group: 'View',
  noShortcutReason: 'Reached from the view toolbar and the command menu.',
  when: onView,
  run: emit('view.duplicate'),
});
