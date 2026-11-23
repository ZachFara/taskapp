import { defineCommand } from '../../registry';
import { moveTo, type Move, type Target } from '../../../rowActions';
import { askRename } from '../../../rowUi.svelte';
import { choices } from '../../../folderTree';
import type { FolderSection } from '../../../bindings';
import type { Ctx } from '../../registry';

type Open = { section: FolderSection; target: Target };

/** The view or project the route shows, as a row target. Cycles have no page of their own. */
async function openItem(ctx: Ctx): Promise<Open | null> {
  const [, kind, key] = ctx.route.split('/');
  if (kind === 'view') {
    const v = (await ctx.api.views.list()).find((x) => x.id === Number(key));
    return v ? { section: 'view', target: { kind: 'view', id: v.id, name: v.name } } : null;
  }
  if (kind === 'project') {
    const p = (await ctx.api.projects.list()).find((x) => x.slug === key);
    return p ? { section: 'project', target: { kind: 'project', id: p.id, name: p.name } } : null;
  }
  return null;
}

// The open view or project moves to a folder of its section, to the top level, or to a new folder.
defineCommand({
  id: 'item.moveToFolder', label: 'Move to folder…', icon: 'folder', group: 'View',
  noShortcutReason: 'Rare action, reached from the command menu; dragging a sidebar row does the same.',
  when: (ctx) => /^\/(view|project)\/[^/]+/.test(ctx.route),
  page: (ctx) => ({
    title: 'Move to folder',
    items: Promise.all([openItem(ctx), ctx.api.folders.list()]).then(([open, fs]) => {
      if (!open) return [];
      const go = (folder: Move['folder']) => async () => {
        const made = await moveTo({ ...open.target, folder });
        if (made) askRename(made);
      };
      const names = new Map(fs.map((f) => [f.id, f]));
      const path = (id: number): string => { const f = names.get(id)!; return f.parent_id === null ? f.name : `${path(f.parent_id)} / ${f.name}`; };
      return [
        { id: 'top', label: 'Top level', icon: 'layers', run: go(null) },
        ...choices(fs, open.section).map((c) => ({ id: `f${c.id}`, label: path(c.id), icon: 'folder', run: go(c.id) })),
        { id: 'new', label: 'New folder…', icon: 'plus', run: go('new') },
      ];
    }),
  }),
});
