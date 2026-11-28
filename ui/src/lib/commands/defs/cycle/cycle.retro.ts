import { defineCommand } from '../../registry';

defineCommand({
  id: 'cycle.retro',
  label: 'Open cycle retrospective…',
  icon: 'cycle',
  group: 'Cycle',
  noShortcutReason: 'reached from the command menu and the Dashboard when a cycle is chosen',
  keywords: ['retrospective', 'review', 'goal', 'rating'],
  page: (ctx) => ({
    title: 'Cycle retrospective',
    items: ctx.api.cycles.list().then((cycles) =>
      [...cycles].reverse().map((c) => ({ id: String(c.id), label: c.name ?? `Cycle ${c.number}`, entity: { icon: c.icon, color: c.color }, run: () => ctx.navigate(`/cycles#${c.number}`) })),
    ),
  }),
});
