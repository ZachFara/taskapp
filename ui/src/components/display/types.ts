// The display settings W1a adds to ViewDef. Until the generated bindings carry them they are read
// defensively from here, so every layout asks this file instead of touching `def.density` directly.
import { api } from '../../lib/api';
import type { Cycle, FieldRef, FixedField, IssueStats, Label, Milestone, Project, PropertyDef, PropertyKind, Status, ViewDef } from '../../lib/bindings';
import { workspaceZone } from '../views/dates';
import { fixedOn } from '../../lib/fields.svelte';
import { pointsOn, readEstimation } from '../insights/estimation';

export type Density = 'comfortable' | 'compact';
export type SubIssues = 'nest' | 'flat' | 'hide';
export type DisplayDef = ViewDef & {
  density?: Density;
  show_empty_groups?: boolean;
  show_sub_issues?: SubIssues;
  card_fields?: FieldRef[];
  show_weekends?: boolean;
};

// Every built-in field a view can show. Points is hidden by the panel and FieldValue when estimation excludes it.
export const BUILT_IN: { field: FieldRef; name: string }[] = [
  ['Key', 'Key'], ['Priority', 'Priority'], ['Status', 'Status'], ['Label', 'Labels'], ['Project', 'Project'],
  ['Milestone', 'Milestone'], ['Cycle', 'Cycle'], ['Parent', 'Parent'], ['StartDate', 'Start'], ['DueDate', 'Due'], ['Estimate', 'Estimate'], ['Points', 'Points'], ['Depth', 'Depth'],
  ['SubIssueProgress', 'Sub-issues'], ['Checklist', 'Checklist'], ['RelationCount', 'Relations'], ['CommentCount', 'Comments'], ['CreatedAt', 'Created'], ['UpdatedAt', 'Updated'],
].map(([f, name]) => ({ field: { Fixed: f as FixedField }, name }));

export const LIST_DEFAULT: FieldRef[] = ['Label', 'Project', 'DueDate', 'Estimate'].map((f) => ({ Fixed: f as FixedField }));
export const TABLE_DEFAULT: FieldRef[] = ['Priority', 'Title', 'Status', 'Project', 'Label', 'DueDate', 'Estimate', 'Checklist'].map((f) => ({ Fixed: f as FixedField }));
export const BOARD_DEFAULT: FieldRef[] = ['Project', 'Label', 'DueDate', 'Checklist'].map((f) => ({ Fixed: f as FixedField }));

export const defaultFor = (layout: ViewDef['layout']): FieldRef[] =>
  layout === 'List' ? LIST_DEFAULT : layout === 'Board' ? BOARD_DEFAULT : layout === 'Table' ? TABLE_DEFAULT : [];

export const density = (d: ViewDef): Density => (d as DisplayDef).density ?? 'comfortable';
export const showWeekends = (d: ViewDef) => (d as DisplayDef).show_weekends ?? true;
export const showEmptyGroups = (d: ViewDef) => (d as DisplayDef).show_empty_groups ?? false;
export const subIssues = (d: ViewDef): SubIssues => (d as DisplayDef).show_sub_issues ?? 'nest';
/** Fields a layout shows: the view's own list, else the layout default. */
export const stored = (d: ViewDef, card = false): FieldRef[] => (card && (d as DisplayDef).card_fields?.length ? (d as DisplayDef).card_fields : d.columns) ?? [];
export const shown = (d: ViewDef, fallback: FieldRef[], card = false): FieldRef[] => {
  const own = stored(d, card);
  return (own.length ? own : fallback).filter((f) => fixedOn(f.Fixed));
};
export const sameField = (a: FieldRef, b: FieldRef) => a.Property !== undefined ? a.Property === b.Property : a.Fixed === b.Fixed;

export type Ctx = {
  statuses: Status[]; projects: Project[]; cycles: Cycle[]; labels: Label[]; defs: PropertyDef[]; tz: string;
  milestones: Milestone[]; stats: Map<number, IssueStats>;
  /** Whether the `estimation` setting lets points show. */
  points: boolean;
};
export const emptyCtx: Ctx = { statuses: [], projects: [], cycles: [], labels: [], defs: [], tz: 'UTC', milestones: [], stats: new Map(), points: true };

const STAT_FIELDS: (FixedField | undefined)[] = ['SubIssueProgress', 'RelationCount', 'CommentCount'];
const usesStats = (d?: ViewDef) => [...(d?.columns ?? []), ...((d as DisplayDef | undefined)?.card_fields ?? [])].some((f) => STAT_FIELDS.includes(f.Fixed));

/** Everything a field renderer looks up. Counts cost a query, so they load only when the view shows one. */
export async function loadCtx(def?: ViewDef): Promise<Ctx> {
  const [statuses, projects, cycles, labels, defs, tz, stats, est] = await Promise.all([
    api.statuses.list(), api.projects.list(), api.cycles.list(), api.labels.list(), api.properties.listDefs(), workspaceZone(),
    usesStats(def) ? api.views.stats(def!) : Promise.resolve([] as IssueStats[]),
    readEstimation(),
  ]);
  const milestones = (await Promise.all(projects.map((p) => api.projects.listMilestones(p.id)))).flat();
  return { statuses, projects, cycles, labels, defs, tz, milestones, stats: new Map(stats.map((s) => [s.issue_id, s])), points: pointsOn(est.estimation) };
}

// The backend palette keys with their colours, so a stored hex lands on the nearest key (crates/core/src/palette.rs).
const PALETTE: [string, number[]][] = [
  ['gray', [107, 114, 128]], ['red', [239, 68, 68]], ['orange', [249, 115, 22]], ['amber', [245, 158, 11]], ['yellow', [234, 179, 8]],
  ['green', [34, 197, 94]], ['teal', [20, 184, 166]], ['cyan', [6, 182, 212]], ['blue', [59, 130, 246]], ['indigo', [99, 102, 241]],
  ['purple', [168, 85, 247]], ['pink', [236, 72, 153]],
];
/** A palette key for an option colour: keys pass through, a #rrggbb maps to the nearest key, anything else is neutral. */
export function paletteKey(c: string | null | undefined): string | undefined {
  if (!c) return undefined;
  if (/^[a-z]+$/.test(c)) return c;
  const m = /^#([0-9a-f]{6})$/i.exec(c);
  if (!m) return undefined;
  const rgb = [0, 2, 4].map((i) => parseInt(m[1].slice(i, i + 2), 16));
  const dist = (p: number[]) => p.reduce((s, v, i) => s + (v - rgb[i]) ** 2, 0);
  return PALETTE.reduce((a, b) => (dist(b[1]) < dist(a[1]) ? b : a))[0];
}

export const KIND_ICON: Record<PropertyKind, string> = {
  Text: 'list', Number: 'target', Date: 'calendar', Select: 'circle', MultiSelect: 'tag', Checkbox: 'check', Url: 'link', NoteLink: 'bookmark',
};

// Fields each layout already draws in fixed columns, so toggling them on would show them twice.
export const LEADING: FixedField[] = ['Key', 'Priority', 'Status'];

export const fieldName = (f: FieldRef, defs: PropertyDef[]) =>
  f.Property !== undefined ? (defs.find((d) => d.key === f.Property)?.name ?? f.Property) : (BUILT_IN.find((b) => sameField(b.field, f))?.name ?? f.Fixed!);

/** "5 pts" for a group header when the Points field is shown and the rows carry points; else nothing. */
export const pointsTotal = (rows: { points: number | null }[], fields: FieldRef[], ctx: Ctx) => {
  const sum = Math.round(rows.reduce((n, r) => n + (r.points ?? 0), 0) * 10) / 10;
  return ctx.points && sum > 0 && fields.some((f) => f.Fixed === 'Points') ? `${sum} pts` : '';
};
/** Group name for a points value: "3 points", or "No points". */
export const pointsName = (p: number | null) => (p === null ? 'No points' : `${p} points`);
