import { defineCommand } from '../../registry';
import { togglePin, type Target } from '../../../rowActions';

// Row-only: the sidebar passes the folder as the target; the pinned folder then lists its contents under Pinned.
defineCommand({
  id: 'folder.pin', label: 'Pin or unpin folder', icon: 'pin', group: 'View',
  noShortcutReason: 'Reached from the row actions of a folder only.',
  menu: false,
  run: (_c, _n, t) => t && togglePin(t as Target),
});
