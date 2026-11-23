import { defineCommand } from '../../registry';
import { addFolder, type Target } from '../../../rowActions';
import { askRename } from '../../../rowUi.svelte';
import type { FolderSection } from '../../../bindings';

/** The section a folder made from the command menu goes to: the page the user is on. */
const sectionAt = (route: string): FolderSection => (route.startsWith('/project') ? 'project' : route.startsWith('/cycle') ? 'cycle' : 'view');

// Makes the folder, then asks for its name in the rename dialog the row actions use. The sidebar's section menus and
// right-click on a section's empty space pass the section (and the folder to make it in) as the target.
defineCommand({
  id: 'folder.new', label: 'New folder', icon: 'folder', group: 'Create',
  noShortcutReason: 'a button in each sidebar section menu and the command menu',
  run: async (ctx, _n, t) => {
    const at = t as { section: FolderSection; parent?: number | null } | undefined;
    askRename((await addFolder(at?.section ?? sectionAt(ctx.route), at?.parent ?? null)) as Target);
  },
});
