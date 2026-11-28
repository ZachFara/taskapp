import type { Cycle } from '../../../bindings';
import { localDay } from '../../../gantt';
import { workspaceZone } from '../../../../components/views/dates';
import type { Ctx } from '../../registry';

/** The cycle a retrospective command acts on: the one open on the Cycles page (`#<number>`), else the one running today, else the latest. */
export async function target(ctx: Ctx): Promise<Cycle | null> {
  const all = await ctx.api.cycles.list();
  const today = localDay(new Date(), await workspaceZone());
  const open = ctx.route === '/cycles' ? all.find((c) => `#${c.number}` === location.hash) : undefined;
  return open ?? all.find((c) => c.start_date <= today && today <= c.end_date) ?? all.at(-1) ?? null;
}

/** Saves the change to the target cycle; the pages reload on `rows:changed`. */
export async function setRetro(ctx: Ctx, patch: Partial<Cycle>) {
  const c = await target(ctx);
  if (!c) return;
  await ctx.api.cycles.update({ ...c, ...patch });
  dispatchEvent(new CustomEvent('rows:changed'));
}
