import { defineCommand } from '../../registry';
import { emit, onGrouped } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'group.collapse.all',
  label: 'Collapse or expand all groups',
  icon: 'chevron-up',
  group: 'View',
  noShortcutReason: 'Reached from the group header and the command menu.',
  when: onGrouped,
  run: emit('group.collapse.all'),
});
