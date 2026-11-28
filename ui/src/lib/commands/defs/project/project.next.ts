import { defineCommand } from '../../registry';
import { onProject } from '../_event';
import { openProject } from './_project';

// Lists the open issues of the project whose page is open; picking one flags it, "No next action" clears the flag.
defineCommand({
  id: 'project.next', label: 'Set next action', icon: 'flag', group: 'Project',
  noShortcutReason: 'Occasional action; reached from the project page and the command menu.',
  when: onProject,
  page: (ctx) => {
    const set = async (issue: number | null) => {
      const p = await openProject(ctx);
      if (p) await ctx.api.projects.setNext(p.id, issue);
      dispatchEvent(new CustomEvent('rows:changed'));
    };
    return {
      title: 'Next action',
      items: (async () => {
        const p = await openProject(ctx);
        if (!p) return [];
        const [rows, statuses] = await Promise.all([
          ctx.api.views.query({ filter: { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: p.id } }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }),
          ctx.api.statuses.list(),
        ]);
        const open = rows.filter((i) => !['Completed', 'Canceled'].includes(statuses.find((s) => s.id === i.status_id)?.category ?? ''));
        return [{ id: 'none', label: 'No next action', icon: 'close', run: () => set(null) }, ...open.map((i) => ({ id: i.key, label: `${i.key} ${i.title}`, run: () => set(i.id) }))];
      })(),
    };
  },
});
