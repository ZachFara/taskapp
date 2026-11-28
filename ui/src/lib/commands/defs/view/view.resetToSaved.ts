import { defineCommand } from '../../registry';
import { emit, onView } from '../_event';

// Drops the temporary sort and quick filter, back to what the view saved.
defineCommand({
  id: 'view.resetToSaved',
  label: 'Reset to saved',
  icon: 'undo',
  group: 'View',
  noShortcutReason: 'Reached from the view header and the command menu.',
  when: onView,
  run: emit('view.resetToSaved'),
});
