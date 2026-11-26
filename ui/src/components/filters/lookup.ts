import type { Cycle, FieldRef, FixedField, Label, Project, ProjectStatus, RelationKindDef, PropertyDef, PropertyKind, Status, StatusCategory, Value, ViewEntity } from '../../lib/bindings';
import { PRIORITY_OPTIONS } from '../../lib/priority';
import { PROJECT_FIELD_NAMES } from '../../lib/projectViews';
import { fixedKind } from './model';

export type Data = { statuses: Status[]; projects: Project[]; defs: PropertyDef[]; cycles: Cycle[]; labels: Label[]; relationKinds: RelationKindDef[]; projectStatuses?: ProjectStatus[] };
export type Choice = { value: Value; label: string };

const categories: StatusCategory[] = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'];
const names: Record<string, string> = {
  StatusCategory: 'Status category', DueDate: 'Due', StartDate: 'Start', CreatedAt: 'Created', UpdatedAt: 'Updated',
  CompletedAt: 'Completed', RelationKind: 'Relation kind', ProjectStatus: 'Project status', TargetDate: 'Target', IssueCount: 'Issues',
};
const projectCategories = ['Planned', 'Started', 'Paused', 'Completed', 'Cancelled'];

/** Names, value kinds and pick-lists for any field, from the loaded workspace data. */
export function makeLookup(d: Data, entity: ViewEntity = 'issue') {
  const fixedName = (f: FixedField) => (entity === 'project' ? PROJECT_FIELD_NAMES[f] : undefined) ?? names[f] ?? f;
  const name = (f: FieldRef) =>
    f.Property !== undefined ? (d.defs.find((p) => p.key === f.Property)?.name ?? f.Property) : fixedName(f.Fixed!);
  const kind = (f: FieldRef): PropertyKind =>
    f.Property !== undefined ? (d.defs.find((p) => p.key === f.Property)?.kind ?? 'Text') : fixedKind(f.Fixed!);
  const named = (rows: { id: number; name: string }[]): Choice[] => rows.map((r) => ({ value: r.id, label: r.name }));
  const choices = (f: FieldRef): Choice[] => {
    if (f.Property !== undefined) return (d.defs.find((p) => p.key === f.Property)?.options ?? []).map((o) => ({ value: o.value, label: o.value }));
    const project = entity === 'project';
    switch (f.Fixed) {
      case 'Status': return named(project ? (d.projectStatuses ?? []) : d.statuses);
      case 'ProjectStatus': return named(d.projectStatuses ?? []);
      case 'Project': return named(d.projects);
      case 'Cycle': return named(d.cycles.map((c) => ({ id: c.id, name: c.name ?? `${c.start_date} to ${c.end_date}` })));
      case 'Label': return named(d.labels);
      case 'RelationKind': return d.relationKinds.map((k) => ({ value: k.id, label: k.name }));
      case 'StatusCategory': return (project ? projectCategories : categories).map((c) => ({ value: c.toLowerCase(), label: c })); // stored lowercase in the db
      case 'Priority': return PRIORITY_OPTIONS.map(({ value, name }) => ({ value, label: name }));
      default: return [];
    }
  };
  return { name, kind, choices };
}
export type Lookup = ReturnType<typeof makeLookup>;
export const fieldKey = (f: FieldRef) => (f.Property !== undefined ? `P:${f.Property}` : `F:${f.Fixed}`);
export const parseField = (k: string): FieldRef => (k.startsWith('P:') ? { Property: k.slice(2) } : { Fixed: k.slice(2) as never });
