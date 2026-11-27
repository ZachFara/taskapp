import type { Project } from '../../../bindings';
import type { Ctx } from '../../registry';
import { patch } from '../../../rowActions';

/** The project whose page is open. */
export async function openProject(ctx: Ctx): Promise<Project | undefined> {
  const slug = ctx.route.split('/')[2];
  return (await ctx.api.projects.list()).find((p) => p.slug === slug);
}

/** One update of the open project; the page and the sidebar reload on `rows:changed`. */
export async function setProject(ctx: Ctx, change: (p: Project) => Parameters<typeof patch>[1]) {
  const p = await openProject(ctx);
  if (p) await patch({ kind: 'project', id: p.id, name: p.name }, change(p));
}
