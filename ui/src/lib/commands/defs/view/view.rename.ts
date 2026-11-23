import { defineCommand } from '../../registry';
import { askRename } from '../../../rowUi.svelte';
import { emit, onView } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'view.rename',
  label: 'Rename view',
  icon: 'edit',
  group: 'View',
  noShortcutReason: 'Reached from the view toolbar and the command menu.',
  when: onView,
  run: (ctx, _n, t) => (t ? askRename(t as Target) : emit('view.rename')(ctx)),
});
