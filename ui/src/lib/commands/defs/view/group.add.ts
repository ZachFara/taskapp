import { defineCommand } from '../../registry';
import { emit, onGrouped } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'group.add',
  label: 'Add issue to this group',
  icon: 'plus',
  group: 'View',
  noShortcutReason: 'Reached from the group header and the command menu.',
  when: onGrouped,
  run: emit('group.add'),
});
