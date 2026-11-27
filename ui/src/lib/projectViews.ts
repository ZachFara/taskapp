// What a project view shows: its fields, its groups and the display lists, as pure functions so the five layouts stay thin.
import type { FieldRef, FixedField, Label, Project, ProjectRow, ProjectStatus, PropertyDef, ViewDef } from './bindings';
import { priorityName } from './priority';
import { glyphCategory } from './projectFields';

export type PCtx = { statuses: ProjectStatus[]; labels: Label[]; defs: PropertyDef[] };
export const emptyPCtx: PCtx = { statuses: [], labels: [], defs: [] };

/** Names of the fixed fields a project view can filter, group, sort and show. */
export const PROJECT_FIELD_NAMES: Partial<Record<FixedField, string>> = {
  Title: 'Name', Status: 'Status', StatusCategory: 'Status category', Priority: 'Priority', Label: 'Labels', StartDate: 'Start',
  TargetDate: 'Target', Progress: 'Progress', ProgressChart: 'Progress chart', IssueCount: 'Issues', Archived: 'Archived',
};
/** The fields the Display panel lists, in order. */
export const PROJECT_BUILT_IN: { field: FieldRef; name: string }[] = (['Status', 'Priority', 'Label', 'Progress', 'ProgressChart', 'IssueCount', 'StartDate', 'TargetDate'] as FixedField[])
  .map((f) => ({ field: { Fixed: f }, name: PROJECT_FIELD_NAMES[f]! }));

const fixed = (...f: FixedField[]): FieldRef[] => f.map((x) => ({ Fixed: x }));
/** Fields a layout shows when the view lists none of its own. */
export const PROJECT_DEFAULT = {
  List: fixed('Label', 'Progress', 'TargetDate'),
  Board: fixed('Label', 'Progress', 'TargetDate'),
  Table: fixed('Status', 'Priority', 'Label', 'Progress', 'TargetDate', 'IssueCount'),
  Gantt: [] as FieldRef[],
  Calendar: [] as FieldRef[],
};

export const fieldName = (f: FieldRef, defs: PropertyDef[]) =>
  f.Property !== undefined ? (defs.find((d) => d.key === f.Property)?.name ?? f.Property) : (PROJECT_FIELD_NAMES[f.Fixed!] ?? f.Fixed!);

/** The fields to draw: the view's own list (columns; a board's card fields when it has them), else the layout's default. */
export function shownFields(def: ViewDef, card = false): FieldRef[] {
  const own = card && def.card_fields?.length ? def.card_fields : def.columns;
  return own.length ? own : PROJECT_DEFAULT[def.layout];
}

export async function loadPCtx(load: { statuses: () => Promise<ProjectStatus[]>; labels: () => Promise<Label[]>; defs: () => Promise<PropertyDef[]> }): Promise<PCtx> {
  const [statuses, labels, defs] = await Promise.all([load.statuses(), load.labels(), load.defs()]);
  return { statuses, labels, defs: defs.filter((d) => d.applies_to !== 'Issues') };
}

export type Group = {
  key: string;
  name: string;
  status: ProjectStatus | null;
  /** A priority group's level, for its glyph. */
  priority: number | null;
  rows: ProjectRow[];
  /** What dropping a project here sets. */
  patch: (p: Project) => Project;
};

/** Group fields a project view offers: a status, a priority, or a select property. */
export const groupable = (f: FieldRef | null, defs: PropertyDef[]) =>
  f !== null && (f.Fixed === 'Status' || f.Fixed === 'StatusCategory' || f.Fixed === 'Priority' || defs.some((d) => d.key === f.Property && d.kind === 'Select'));

/**
 * Rows split by the group field, in the order of the field's values; `null` is one nameless group.
 * Empty groups are dropped unless `empty` (a board keeps every column). A field that cannot group counts as status when `fallback`.
 */
export function groupRows(rows: ProjectRow[], by: FieldRef | null, ctx: PCtx, opts: { empty?: boolean; fallback?: boolean } = {}): Group[] {
  const field = groupable(by, ctx.defs) ? by : opts.fallback ? { Fixed: 'Status' as const } : null;
  const keep = (g: Group[]) => (opts.empty ? g : g.filter((x) => x.rows.length));
  if (!field) return [{ key: 'all', name: '', status: null, priority: null, rows, patch: (p) => p }];
  const make = (key: string, name: string, hold: (r: ProjectRow) => boolean, patch: (p: Project) => Project, extra: Partial<Group> = {}): Group =>
    ({ key, name, status: null, priority: null, rows: rows.filter(hold), patch, ...extra });
  if (field.Fixed === 'Priority') {
    return keep([1, 2, 3, 4, 0].map((n) => make(`p${n}`, priorityName(n), (r) => r.project.priority === n, (p) => ({ ...p, priority: n }), { priority: n })));
  }
  if (field.Fixed === 'Status') {
    return keep([...ctx.statuses].sort((a, b) => a.position - b.position).map((s) => make(`s${s.id}`, s.name, (r) => r.project.status_id === s.id, (p) => ({ ...p, status_id: s.id }), { status: s })));
  }
  if (field.Fixed === 'StatusCategory') {
    // Dropping into a category sets the first status of that category.
    const cats = [...new Set([...ctx.statuses].sort((a, b) => a.position - b.position).map((s) => s.category))];
    return keep(cats.map((c) => {
      const first = ctx.statuses.filter((s) => s.category === c).sort((a, b) => a.position - b.position)[0];
      return make(`c${c}`, c, (r) => ctx.statuses.find((s) => s.id === r.project.status_id)?.category === c, (p) => ({ ...p, status_id: first.id }), { status: first });
    }));
  }
  const def = ctx.defs.find((d) => d.key === field.Property)!;
  const value = (r: ProjectRow) => {
    const v = r.project.properties[def.key];
    return typeof v === 'string' && def.options.some((o) => o.value === v) ? v : null;
  };
  const set = (v: string | null) => (p: Project): Project => ({ ...p, properties: { ...p.properties, [def.key]: v } });
  return keep([...def.options.map((o) => make(`o${o.value}`, o.value, (r) => value(r) === o.value, set(o.value))), make('none', 'None', (r) => value(r) === null, set(null))]);
}

export const statusGlyph = (s: ProjectStatus | undefined) => (s ? glyphCategory[s.category] : 'Backlog');

/** The percent of a project's issues that are done (zero without issues). */
export const percent = (r: ProjectRow) => (r.issue_total ? Math.round((r.issue_done / r.issue_total) * 100) : 0);

/** A project's dates as calendar events: its start and its target day. */
export function dateChips(rows: ProjectRow[]) {
  return rows.flatMap((r) => [
    ...(r.project.start_date ? [{ day: r.project.start_date, kind: 'start' as const, row: r }] : []),
    ...(r.project.target_date ? [{ day: r.project.target_date, kind: 'target' as const, row: r }] : []),
  ]);
}

/** The days a month grid shows: whole weeks from the week start before the 1st to the week end after the last day. */
export function monthDays(year: number, month: number, monday: boolean): { day: string; inMonth: boolean }[] {
  const first = Date.UTC(year, month, 1);
  const lead = (new Date(first).getUTCDay() - (monday ? 1 : 0) + 7) % 7;
  const last = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  const total = Math.ceil((lead + last) / 7) * 7;
  return Array.from({ length: total }, (_, k) => {
    const d = new Date(first + (k - lead) * 86_400_000);
    return { day: d.toISOString().slice(0, 10), inMonth: d.getUTCMonth() === month };
  });
}
