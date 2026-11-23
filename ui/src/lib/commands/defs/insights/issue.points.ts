import { readEstimation, pointsOn, SCALES, type Estimation } from '../../../../components/insights/estimation';
import type { IssuePatch } from '../../../bindings';
import { defineCommand } from '../../registry';
import { apply, patchOf } from '../issue/_issue';

const set = (ctx: Parameters<typeof apply>[0], points: number | null) => apply(ctx, patchOf({ points } as IssuePatch));

// `when` is synchronous, so the setting is cached and re-read after every write (a settings change is one).
let estimation: Estimation = 'both';
// A test api without settings, or a failing read, counts as the default (both): `when` must never throw.
const refresh = () => {
  try {
    void readEstimation().then((r) => (estimation = r.estimation), () => {});
  } catch {
    /* no settings api */
  }
};
refresh();
addEventListener('history:wrote', refresh);

defineCommand({
  id: 'issue.points', label: 'Set points', icon: 'bolt', group: 'Issue', shortcut: 'shift+e',
  when: () => pointsOn(estimation),
  page: (ctx) => ({
    title: 'Points',
    // A typed number to one decimal; `none` clears.
    parse: (text) => {
      const t = text.trim().toLowerCase();
      if (t === 'none') return { label: 'Clear the points', run: () => set(ctx, null) };
      if (!/^\d+(\.\d+)?$/.test(t)) return null;
      const n = Math.round(Number(t) * 10) / 10;
      return { label: `Set the points to ${n}`, run: () => set(ctx, n) };
    },
    items: readEstimation().then(({ estimation, scale }) =>
      pointsOn(estimation)
        ? [...SCALES[scale].map((s) => ({ id: s.label, label: s.label, run: () => set(ctx, s.value) })), { id: 'none', label: 'No points', run: () => set(ctx, null) }]
        : [{ id: 'off', label: 'Points are off: choose Points or Both in Settings', run: () => ctx.navigate('/settings/general') }],
    ),
  }),
});
