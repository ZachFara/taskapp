import { defineCommand } from '../../registry';
import { PRIORITY_OPTIONS } from '../../../priority';
import { onProject } from '../_event';
import { setProject } from './_project';

defineCommand({
  id: 'project.priority', label: 'Set project priority', icon: 'flag', group: 'Project',
  noShortcutReason: 'Occasional action; reached from the project page and the command menu.',
  when: onProject,
  page: (ctx) => ({
    title: 'Project priority',
    items: PRIORITY_OPTIONS.map(({ name, value }) => ({ id: String(value), label: name, run: () => setProject(ctx, () => ({ priority: value })) })),
  }),
});
