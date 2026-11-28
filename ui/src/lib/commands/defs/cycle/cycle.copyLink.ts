import { defineCommand } from '../../registry';
import { copyLink } from '../../../deepLink';
import type { Target } from '../../../rowActions';
import { target } from './_retro';

defineCommand({
  id: 'cycle.copyLink',
  label: 'Copy link to cycle',
  icon: 'link',
  group: 'Cycle',
  noShortcutReason: 'Rare action, reached from the right-click menu and the command menu.',
  when: (ctx) => ctx.route === '/cycles',
  run: async (ctx, _n, t) => {
    const id = (t as Target | undefined)?.id;
    const c = id === undefined ? await target(ctx) : (await ctx.api.cycles.list()).find((x) => x.id === id);
    if (c) await copyLink('cycle', c.number);
  },
});
