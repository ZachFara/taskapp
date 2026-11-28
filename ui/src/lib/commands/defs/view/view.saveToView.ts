import { defineCommand } from '../../registry';
import { emit, onView } from '../_event';

// The view page keeps its temporary sort and quick filter in the saved view.
defineCommand({
  id: 'view.saveToView',
  label: 'Save to view',
  icon: 'check',
  group: 'View',
  noShortcutReason: 'Reached from the view header and the command menu.',
  when: onView,
  run: emit('view.saveToView'),
});
