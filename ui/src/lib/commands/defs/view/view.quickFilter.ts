import { defineCommand } from '../../registry';
import { emit, onView } from '../_event';

// The view page owns the quick filter bar; Esc inside it clears and closes it.
defineCommand({
  id: 'view.quickFilter',
  label: 'Show or hide quick filter',
  icon: 'filter',
  group: 'View',
  shortcut: 'shift+f',
  when: onView,
  run: emit('view.quickFilter'),
});
