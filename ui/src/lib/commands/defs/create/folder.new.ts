import { defineCommand } from '../../registry';
import { addFolder } from '../../../rowActions';
import { askRename } from '../../../rowUi.svelte';

// Makes the folder, then asks for its name in the rename dialog the row actions use.
defineCommand({
  id: 'folder.new', label: 'New folder', icon: 'folder', group: 'Create',
  noShortcutReason: 'a button in the sidebar Views menu and the command menu',
  run: async () => askRename(await addFolder()),
});
