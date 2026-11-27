import { defineCommand } from '../../registry';
import { onProject } from '../_event';
import { setProject } from './_project';

defineCommand({
  id: 'project.status', label: 'Set project status', icon: 'circle', group: 'Project',
  noShortcutReason: 'Occasional action; reached from the project page and the command menu.',
  when: onProject,
  page: (ctx) => ({
    title: 'Project status',
    items: ctx.api.project.statusesList().then((list) =>
      list.map((s) => ({ id: String(s.id), label: s.name, run: () => setProject(ctx, () => ({ status_id: s.id })) }))),
  }),
});
