import { defineCommand } from '../../registry';
import { togglePin, type Target } from '../../../rowActions';

// Row-only: the sidebar and the cycles page pass the cycle as the target.
defineCommand({
  id: 'cycle.pin',
  label: 'Pin or unpin cycle',
  icon: 'pin',
  group: 'Cycle',
  noShortcutReason: 'Reached from the row actions of a cycle only.',
  menu: false,
  run: (_c, _n, t) => t && togglePin(t as Target),
});
