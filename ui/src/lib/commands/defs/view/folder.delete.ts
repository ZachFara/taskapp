import { defineCommand } from '../../registry';
import { askDelete } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

defineCommand({
  id: 'folder.delete', label: 'Delete folder', icon: 'trash', group: 'View',
  noShortcutReason: 'Reached from the row actions of a folder only.',
  menu: false,
  run: (_c, _n, t) => t && askDelete(t as Target),
});
