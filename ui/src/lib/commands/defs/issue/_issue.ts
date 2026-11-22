import type { Api } from '../../../api';
import type { BulkPatch, IssuePatch, Cycle, Issue, Label, Project, PropertyDef, Status } from '../../../bindings';
import { formatDay, formatEstimate, parseDate, parseEstimate } from '../../../parse/when';
import type { CommandPage, Ctx } from '../../registry';

/** What the pages list; filled from the api each time a page opens, so a page never lists stale or empty choices. */
export const choices = {
  statuses: [] as Status[], projects: [] as Project[], labels: [] as Label[],
  cycles: [] as Cycle[], defs: [] as PropertyDef[], issues: [] as Issue[],
  /** The workspace time zone, which decides what "today" is for the date pages. */
  tz: Intl.DateTimeFormat().resolvedOptions().timeZone,
};

const noView = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false } as const;

export async function loadChoices(api: Api) {
  const [statuses, projects, labels, cycles, defs, issues, workspace] = await Promise.all([
    api.statuses.list(), api.projects.list(), api.labels.list(), api.cycles.list(), api.properties.listDefs(),
    api.views.query(noView), api.workspace.status(),
  ]);
  Object.assign(choices, { statuses, projects, labels, cycles, defs, issues, tz: workspace.timezone || choices.tz });
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

type Row = [label: string, patch: BulkPatch, icon?: string];
/**
 * A page whose items each apply one bulk patch. Rows given as a function are built after the choices have loaded and
 * the items come back as a Promise; fixed rows are listed at once.
 */
export function pageOf(ctx: Ctx, title: string, rows: Row[] | (() => Row[]), extra: Pick<CommandPage, 'parse'> = {}): CommandPage {
  const list = (r: Row[]) => r.map(([label, p, icon]) => ({ id: label, label, icon, run: () => apply(ctx, p) }));
  return { title, ...extra, items: typeof rows === 'function' ? loadChoices(ctx.api).then(() => list(rows())) : list(rows) };
}

/** Typed text on the due or start date page: a day, or `none` to clear it. */
export const dateParse = (ctx: Ctx, field: 'due' | 'start_date'): CommandPage['parse'] => (text) => {
  const r = parseDate(text, choices.tz);
  if (!r) return null;
  const clear = r.value === null;
  const patch = field === 'due' ? { due: clear ? null : { Date: r.value! } } : { start_date: r.value };
  const what = field === 'due' ? 'due date' : 'start date';
  return { label: clear ? `Clear the ${what}` : `Set the ${what} to ${formatDay(r.value!)}`, run: () => apply(ctx, patchOf(patch)) };
};

/** Typed text on the estimate page: `30m`, `1h30m`, `1.5h`, `1d` (8 hours), or `none`. */
export const estimateParse = (ctx: Ctx): CommandPage['parse'] => (text) => {
  const r = parseEstimate(text);
  if (!r) return null;
  const label = r.value === null ? 'Clear the estimate' : `Set the estimate to ${formatEstimate(r.value)}`;
  return { label, run: () => apply(ctx, patchOf({ estimate_minutes: r.value })) };
};

/** The day `plus` days from today in the workspace zone. */
export const day = (plus = 0) => parseDate(plus ? `+${plus}d` : 'today', choices.tz)!.value!;

/** Archive is the only removal; more than this many issues asks first. */
export const CONFIRM_ABOVE = 5;
export async function archive(ctx: Ctx) {
  // A big selection asks through the command menu's confirm page (issue.archive.confirm), never the browser's dialog.
  if (targets(ctx).length > CONFIRM_ABOVE) return void dispatchEvent(new CustomEvent('command:palette.open', { detail: { page: 'issue.archive.confirm' } }));
  await apply(ctx, patchOf({}, { archive: true }));
}

export const keyOf = async (ctx: Ctx) => (await ctx.api.issues.get(targets(ctx)[0])).key;
