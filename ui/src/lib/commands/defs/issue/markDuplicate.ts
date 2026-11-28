import { defineCommand } from '../../registry';
import { loadChoices, refreshView, choices, targets } from './_issue';

// The original is picked from the issues most like this one's title first, then any other issue.
defineCommand({
  id: 'issue.markDuplicate', label: 'Mark as duplicate of…', icon: 'copy', group: 'Issue',
  noShortcutReason: 'Reached from the command menu and the right-click menu.',
  when: (ctx) => targets(ctx).length === 1,
  page: (ctx) => ({
    title: 'Duplicate of',
    items: (async () => {
      const [self] = targets(ctx);
      const [issue] = await Promise.all([ctx.api.issues.get(self), loadChoices(ctx.api)]);
      const similar = (await ctx.api.search.similar(issue.title, issue.project_id ?? null, 6).catch(() => [])).filter((i) => i.id !== self);
      const rest = choices.issues.filter((i) => i.id !== self && !similar.some((s) => s.id === i.id));
      return [...similar, ...rest].map((i) => ({
        id: i.key, label: `${i.key} ${i.title}`,
        run: async () => {
          await ctx.api.issues.markDuplicate(self, i.id);
          refreshView();
        },
      }));
    })(),
  }),
});
