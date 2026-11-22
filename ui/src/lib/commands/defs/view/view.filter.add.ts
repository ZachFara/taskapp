import { defineCommand } from '../../registry';
import { emit, onView } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'view.filter.add',
  label: 'Add filter',
  icon: 'filter',
  group: 'View',
  shortcut: 'f',
  when: onView,
  run: emit('view.filter.add'),
});
