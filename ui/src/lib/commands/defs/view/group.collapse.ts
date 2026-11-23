import { defineCommand } from '../../registry';
import { emit, onGrouped } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'group.collapse',
  label: 'Collapse or expand group',
  icon: 'chevron-down',
  group: 'View',
  noShortcutReason: 'Reached from the group header and the command menu.',
  when: onGrouped,
  run: emit('group.collapse'),
});
