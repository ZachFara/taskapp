import { defineCommand } from '../../registry';
import { remove, type Target } from '../../../rowActions';

defineCommand({
  id: 'pin.unpin', label: 'Unpin', icon: 'close', group: 'View',
  noShortcutReason: 'Reached from the row actions of a pinned row only.',
  menu: false,
  run: (_c, _n, t) => t && remove(t as Target),
});
