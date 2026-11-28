import { dashboard, set } from '../../../../components/dashboard/dashboard.svelte';
import { defineCommand } from '../../registry';

defineCommand({
  id: 'dashboard.filter',
  label: 'Dashboard project or label…',
  icon: 'filter',
  group: 'View',
  noShortcutReason: 'reached from the dashboard filters and the command menu',
  when: (ctx) => ctx.route === '/dashboard',
  page: (ctx) => ({
    title: 'Dashboard filter',
    items: Promise.all([ctx.api.projects.list(), ctx.api.labels.list()]).then(([projects, labels]) => [
      { id: 'clear', label: 'All projects and labels', icon: 'close', run: () => set({ project: null, label: null }) },
      ...projects.map((p) => ({ id: `p${p.id}`, label: p.name, entity: { icon: p.icon, color: p.color }, checked: () => dashboard.project === p.id, run: () => set({ project: dashboard.project === p.id ? null : p.id }) })),
      ...labels.map((l) => ({ id: `l${l.id}`, label: l.name, icon: 'tag', checked: () => dashboard.label === l.id, run: () => set({ label: dashboard.label === l.id ? null : l.id }) })),
    ]),
  }),
});
