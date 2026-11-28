import { defineCommand } from '../../registry';
import { copyLink } from '../../../deepLink';
import { onProject } from '../_event';
import { openProject } from './_project';

defineCommand({
  id: 'milestone.copyLink',
  label: 'Copy link to milestone',
  icon: 'link',
  group: 'Project',
  noShortcutReason: 'Rare action, reached from the command menu on the project page.',
  when: onProject,
  page: (ctx) => ({
    title: 'Milestone',
    items: openProject(ctx).then(async (p) =>
      p ? (await ctx.api.projects.listMilestones(p.id)).map((m) => ({ id: String(m.id), label: m.name, icon: 'target', run: () => copyLink('milestone', m.id) })) : [],
    ),
  }),
});
