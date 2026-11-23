import { defineCommand } from '../../registry';
import { addFolder, type Target } from '../../../rowActions';
import { askRename } from '../../../rowUi.svelte';
import { folders } from '../../../stores.svelte';

defineCommand({
  id: 'folder.newInside', label: 'New folder inside', icon: 'folder', group: 'View',
  noShortcutReason: 'Reached from the row actions of a folder only.',
  menu: false,
  run: async (_c, _n, t) => {
    const f = folders.find((x) => x.id === (t as Target)?.id);
    if (f) askRename(await addFolder(f.section, f.id));
  },
});
