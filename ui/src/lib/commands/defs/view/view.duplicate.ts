import { defineCommand } from '../../registry';
import { duplicate, type Target } from '../../../rowActions';
import { emit, onView } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'view.duplicate',
  label: 'Save as new view',
  icon: 'copy',
  group: 'View',
  noShortcutReason: 'Reached from the view toolbar and the command menu.',
  when: onView,
  run: (ctx, _n, t) => (t ? duplicate(t as Target) : emit('view.duplicate')(ctx)),
});
