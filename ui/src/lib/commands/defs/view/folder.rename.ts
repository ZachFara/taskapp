import { defineCommand } from '../../registry';
import { askRename } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the sidebar passes the folder as the target.
defineCommand({
  id: 'folder.rename', label: 'Rename folder', icon: 'edit', group: 'View',
  noShortcutReason: 'Reached from the row actions of a folder only.',
  menu: false,
  run: (_c, _n, t) => t && askRename(t as Target),
});
