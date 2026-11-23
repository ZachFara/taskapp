import { defineCommand } from '../../registry';
import { onView } from '../_event';
import { refreshAll } from '../../../stores.svelte';

// The open view moves to a folder, or out to the top level. Dragging a row in the sidebar does the same.
defineCommand({
  id: 'view.moveToFolder', label: 'Move view to folder', icon: 'folder', group: 'View',
  noShortcutReason: 'Rare action, reached from the command menu.',
  when: onView,
  page: (ctx) => {
    const id = Number(ctx.route.split('/')[2]);
    const move = (folder: number | null) => async () => {
      await ctx.api.views.move(id, folder);
      await refreshAll();
      dispatchEvent(new CustomEvent('rows:changed'));
    };
    return {
      title: 'Move view to folder',
      items: ctx.api.folders.list().then((fs) => [
        { id: 'top', label: 'No folder (top level)', icon: 'layers', run: move(null) },
        ...fs.map((f) => ({ id: `f${f.id}`, label: f.name, icon: 'folder', run: move(f.id) })),
      ]),
    };
  },
});
