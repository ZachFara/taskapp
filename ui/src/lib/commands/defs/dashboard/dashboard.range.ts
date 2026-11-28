import { dashboard, set } from '../../../../components/dashboard/dashboard.svelte';
import { PRESETS } from '../../../../components/dashboard/metrics';
import { defineCommand } from '../../registry';

const DAY = /^\d{4}-\d{2}-\d{2}$/;

defineCommand({
  id: 'dashboard.range',
  label: 'Dashboard range…',
  icon: 'calendar',
  group: 'View',
  noShortcutReason: 'reached from the dashboard range picker and the command menu',
  keywords: ['today', 'week', 'cycle', 'month', 'quarter', 'year', 'custom', 'dates'],
  when: (ctx) => ctx.route === '/dashboard',
  page: (ctx) => ({
    title: 'Dashboard range',
    // Two dates typed as `2026-11-01 2026-11-30` make a custom range.
    parse: (text) => {
      const [from, to] = text.trim().split(/\s+/);
      return DAY.test(from ?? '') && DAY.test(to ?? '') ? { label: `Custom range ${from} to ${to}`, run: () => set({ range: 'custom', from, to }) } : null;
    },
    items: ctx.api.cycles.list().then((cycles) => [
      ...PRESETS.filter((p) => p.id !== 'custom').map((p) => ({ id: p.id, label: p.label, icon: 'calendar', checked: () => dashboard.range === p.id, run: () => set({ range: p.id }) })),
      ...cycles.map((c) => ({ id: `cycle:${c.id}`, label: c.name ?? `Cycle ${c.number}`, entity: { icon: c.icon, color: c.color }, checked: () => dashboard.range === `cycle:${c.id}`, run: () => set({ range: `cycle:${c.id}` }) })),
    ]),
  }),
});
