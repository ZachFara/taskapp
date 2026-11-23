import { defineCommand } from '../../registry';
import { moveTo, type Move } from '../../../rowActions';
import { askRename } from '../../../rowUi.svelte';

// The "Move to folder" submenu of a row and a drop in the sidebar both run this, with the row, the folder (or the top
// level) and the place among its siblings.
defineCommand({
  id: 'folder.move', label: 'Move to folder', icon: 'folder', group: 'View',
  noShortcutReason: 'Reached from the row menu and by dragging; the command menu has "Move to folder…".',
  menu: false,
  run: async (_c, _n, t) => {
    const made = t && (await moveTo(t as Move));
    if (made) askRename(made);
  },
});
