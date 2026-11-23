import type { Cycle, FieldRef, Label, Project, RelationKindDef, PropertyDef, PropertyKind, Status, StatusCategory, Value } from '../../lib/bindings';
import { PRIORITY_OPTIONS } from '../../lib/priority';
import { fixedKind } from './model';

export type Data = { statuses: Status[]; projects: Project[]; defs: PropertyDef[]; cycles: Cycle[]; labels: Label[]; relationKinds: RelationKindDef[] };
export type Choice = { value: Value; label: string };

const categories: StatusCategory[] = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'];
const names: Record<string, string> = {
  StatusCategory: 'Status category', DueDate: 'Due', StartDate: 'Start', CreatedAt: 'Created', UpdatedAt: 'Updated',
  CompletedAt: 'Completed', RelationKind: 'Relation kind',
};

/** Names, value kinds and pick-lists for any field, from the loaded workspace data. */
export function makeLookup(d: Data) {
  const name = (f: FieldRef) =>
    f.Property !== undefined ? (d.defs.find((p) => p.key === f.Property)?.name ?? f.Property) : (names[f.Fixed!] ?? f.Fixed!);
  const kind = (f: FieldRef): PropertyKind =>
    f.Property !== undefined ? (d.defs.find((p) => p.key === f.Property)?.kind ?? 'Text') : fixedKind(f.Fixed!);
  const named = (rows: { id: number; name: string }[]): Choice[] => rows.map((r) => ({ value: r.id, label: r.name }));
  const choices = (f: FieldRef): Choice[] => {
    if (f.Property !== undefined) return (d.defs.find((p) => p.key === f.Property)?.options ?? []).map((o) => ({ value: o.value, label: o.value }));
    switch (f.Fixed) {
      case 'Status': return named(d.statuses);
      case 'Project': return named(d.projects);
      case 'Cycle': return named(d.cycles.map((c) => ({ id: c.id, name: c.name ?? `${c.start_date} to ${c.end_date}` })));
      case 'Label': return named(d.labels);
      case 'RelationKind': return d.relationKinds.map((k) => ({ value: k.id, label: k.name }));
      case 'StatusCategory': return categories.map((c) => ({ value: c.toLowerCase(), label: c })) // stored lowercase in the db;
      case 'Priority': return PRIORITY_OPTIONS.map(({ value, name }) => ({ value, label: name }));
      default: return [];
    }
  };
  return { name, kind, choices };
}
export type Lookup = ReturnType<typeof makeLookup>;
export const fieldKey = (f: FieldRef) => (f.Property !== undefined ? `P:${f.Property}` : `F:${f.Fixed}`);
export const parseField = (k: string): FieldRef => (k.startsWith('P:') ? { Property: k.slice(2) } : { Fixed: k.slice(2) as never });
