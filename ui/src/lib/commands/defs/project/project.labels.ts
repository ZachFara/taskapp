import { defineCommand } from '../../registry';
import { onProject } from '../_event';
import { setProject } from './_project';

defineCommand({
  id: 'project.labels', label: 'Add or remove project labels', icon: 'tag', group: 'Project',
  noShortcutReason: 'Occasional action; reached from the project page and the command menu.',
  when: onProject,
  page: (ctx) => ({
    title: 'Project labels',
    items: ctx.api.labels.list().then((labels) => labels.flatMap((l) => [
      { id: `add:${l.id}`, label: `Add ${l.name}`, run: () => setProject(ctx, (p) => ({ label_ids: [...new Set([...p.label_ids, l.id])] })) },
      { id: `remove:${l.id}`, label: `Remove ${l.name}`, run: () => setProject(ctx, (p) => ({ label_ids: p.label_ids.filter((x) => x !== l.id) })) },
    ])),
  }),
});
