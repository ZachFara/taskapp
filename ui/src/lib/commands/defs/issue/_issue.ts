import type { Api } from '../../../api';
import type { BulkPatch, IssuePatch, Cycle, Issue, Label, Project, PropertyDef, Status } from '../../../bindings';
import type { CommandPage, Ctx } from '../../registry';

/** What the sync `page()` lists can offer; filled from the api and refreshed every time a page opens. */
export const choices = {
  statuses: [] as Status[], projects: [] as Project[], labels: [] as Label[],
  cycles: [] as Cycle[], defs: [] as PropertyDef[], issues: [] as Issue[],
};

const noView = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false } as const;

export async function loadChoices(api: Api) {
  const [statuses, projects, labels, cycles, defs, issues] = await Promise.all([
    api.statuses.list(), api.projects.list(), api.labels.list(), api.cycles.list(), api.properties.listDefs(),
    api.views.query(noView),
  ]);
  Object.assign(choices, { statuses, projects, labels, cycles, defs, issues });
}

/** The selection, else the focused issue, else the open one. */
export function targets(ctx: Ctx): number[] {
  if (ctx.selection.length) return ctx.selection;
  const one = ctx.focusedIssueId ?? ctx.openIssueId;
  return one === null ? [] : [one];
}

export const patchOf = (patch: IssuePatch = {}, rest: Omit<BulkPatch, 'patch'> = {}): BulkPatch => ({ patch, ...rest });

/** One backend call, so one undo step, for every target. */
export async function apply(ctx: Ctx, p: BulkPatch) {
  const ids = targets(ctx);
  if (ids.length) await ctx.api.issues.bulkUpdate(ids, p);
}

/** A page whose items each apply one bulk patch; opening it refreshes the lists for next time. */
export function pageOf(ctx: Ctx, title: string, items: [label: string, patch: BulkPatch, icon?: string][]): CommandPage {
  void loadChoices(ctx.api).catch(() => {});
  return { title, items: items.map(([label, p, icon]) => ({ id: label, label, icon, run: () => apply(ctx, p) })) };
}

export const today = (plus = 0) => {
  const d = new Date();
  d.setDate(d.getDate() + plus);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
};

/** Archive is the only removal; more than this many issues asks first. */
export const CONFIRM_ABOVE = 5;
export async function archive(ctx: Ctx) {
  const n = targets(ctx).length;
  if (n > CONFIRM_ABOVE && !confirm(`Archive ${n} issues?`)) return;
  await apply(ctx, patchOf({}, { archive: true }));
}

export const keyOf = async (ctx: Ctx) => (await ctx.api.issues.get(targets(ctx)[0])).key;
