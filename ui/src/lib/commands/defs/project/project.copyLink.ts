import { defineCommand } from '../../registry';
import { copyLink } from '../../../deepLink';
import type { Target } from '../../../rowActions';
import { onProject } from '../_event';
import { openProject } from './_project';

defineCommand({
  id: 'project.copyLink',
  label: 'Copy link to project',
  icon: 'link',
  group: 'Project',
  noShortcutReason: 'Rare action, reached from the project page, the right-click menu and the command menu.',
  when: onProject,
  run: async (ctx, _n, t) => {
    const id = (t as Target | undefined)?.id;
    const p = id === undefined ? await openProject(ctx) : (await ctx.api.projects.list()).find((x) => x.id === id);
    if (p) await copyLink('project', p.slug);
  },
});
