// The display settings W1a adds to ViewDef. Until the generated bindings carry them they are read
// defensively from here, so every layout asks this file instead of touching `def.density` directly.
import { api } from '../../lib/api';
import type { Cycle, FieldRef, FixedField, Label, Project, PropertyDef, Status, ViewDef } from '../../lib/bindings';
import { workspaceZone } from '../views/dates';

export type Density = 'comfortable' | 'compact';
export type SubIssues = 'nest' | 'flat' | 'hide';
export type DisplayDef = ViewDef & {
  density?: Density;
  show_empty_groups?: boolean;
  show_sub_issues?: SubIssues;
  card_fields?: FieldRef[];
  show_weekends?: boolean;
};

// `Key` is shown from the issue itself; the bindings' FixedField has no entry for it yet.
export const BUILT_IN: { field: FieldRef; name: string }[] = [
  ['Key', 'Key'], ['Priority', 'Priority'], ['Status', 'Status'], ['Label', 'Labels'], ['Project', 'Project'],
  ['Cycle', 'Cycle'], ['StartDate', 'Start'], ['DueDate', 'Due'], ['Estimate', 'Estimate'], ['CreatedAt', 'Created'], ['UpdatedAt', 'Updated'],
].map(([f, name]) => ({ field: { Fixed: f as FixedField }, name }));

export const LIST_DEFAULT: FieldRef[] = ['Label', 'Project', 'DueDate', 'Estimate'].map((f) => ({ Fixed: f as FixedField }));
export const TABLE_DEFAULT: FieldRef[] = ['Priority', 'Title', 'Status', 'Project', 'Label', 'DueDate', 'Estimate'].map((f) => ({ Fixed: f as FixedField }));
export const BOARD_DEFAULT: FieldRef[] = ['Project', 'Label', 'DueDate'].map((f) => ({ Fixed: f as FixedField }));

export const defaultFor = (layout: ViewDef['layout']): FieldRef[] =>
  layout === 'List' ? LIST_DEFAULT : layout === 'Board' ? BOARD_DEFAULT : layout === 'Table' ? TABLE_DEFAULT : [];

export const density = (d: ViewDef): Density => (d as DisplayDef).density ?? 'comfortable';
export const showWeekends = (d: ViewDef) => (d as DisplayDef).show_weekends ?? true;
export const showEmptyGroups = (d: ViewDef) => (d as DisplayDef).show_empty_groups ?? false;
export const subIssues = (d: ViewDef): SubIssues => (d as DisplayDef).show_sub_issues ?? 'nest';
/** Fields a layout shows: the view's own list, else the layout default. */
export const shown = (d: ViewDef, fallback: FieldRef[], card = false): FieldRef[] => {
  const own = (card && (d as DisplayDef).card_fields?.length ? (d as DisplayDef).card_fields : d.columns) ?? [];
  return own.length ? own : fallback;
};
export const sameField = (a: FieldRef, b: FieldRef) => a.Property !== undefined ? a.Property === b.Property : a.Fixed === b.Fixed;

export type Ctx = { statuses: Status[]; projects: Project[]; cycles: Cycle[]; labels: Label[]; defs: PropertyDef[]; tz: string };
export const emptyCtx: Ctx = { statuses: [], projects: [], cycles: [], labels: [], defs: [], tz: 'UTC' };
export async function loadCtx(): Promise<Ctx> {
  const [statuses, projects, cycles, labels, defs, tz] = await Promise.all([
    api.statuses.list(), api.projects.list(), api.cycles.list(), api.labels.list(), api.properties.listDefs(), workspaceZone(),
  ]);
  return { statuses, projects, cycles, labels, defs, tz };
}

export const fieldName = (f: FieldRef, defs: PropertyDef[]) =>
  f.Property !== undefined ? (defs.find((d) => d.key === f.Property)?.name ?? f.Property) : (BUILT_IN.find((b) => sameField(b.field, f))?.name ?? f.Fixed!);
