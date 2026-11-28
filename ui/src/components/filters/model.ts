import type { FieldRef, Filter, FixedField, Op, PropertyKind, Value } from '../../lib/bindings';

/** One filter chip: a single condition the user can finish later (an incomplete chip is never saved). */
export type Chip = { id: number; field: FieldRef; op: Op; value: Value };

const text: Op[] = ['Eq', 'Neq', 'Contains', 'IsEmpty', 'IsNotEmpty'];
const ordered: Op[] = ['Eq', 'Neq', 'Lt', 'Lte', 'Gt', 'Gte', 'IsEmpty', 'IsNotEmpty'];
const operators: Record<PropertyKind, Op[]> = {
  Text: text, Url: text, NoteLink: text, Number: ordered, Date: ordered,
  Select: ['Eq', 'Neq', 'In', 'NotIn', 'IsEmpty', 'IsNotEmpty'],
  MultiSelect: ['Contains', 'In', 'IsEmpty', 'IsNotEmpty'],
  Checkbox: ['Eq'],
};
export const opsFor = (kind: PropertyKind) => operators[kind];

/** The relative dates a date filter can pick (core: `today`, `today+N`, `today-N`, counted in the workspace zone). */
export const relativeDays: { value: string; label: string }[] = [
  { value: 'today', label: 'today' }, { value: 'today-7', label: '7 days ago' }, { value: 'today-14', label: '14 days ago' },
  { value: 'today-30', label: '30 days ago' }, { value: 'today+7', label: 'in 7 days' }, { value: 'today+14', label: 'in 14 days' },
  { value: 'today+30', label: 'in 30 days' },
];
export const relativeLabel = (v: Value) => relativeDays.find((r) => r.value === v)?.label;

const fixedKinds: Record<FixedField, PropertyKind> = {
  Title: 'Text', Status: 'Select', StatusCategory: 'Select', Priority: 'Number', Project: 'Select',
  Milestone: 'Number', Cycle: 'Select', Label: 'MultiSelect', Parent: 'Number', StartDate: 'Date',
  DueDate: 'Date', Estimate: 'Number', Points: 'Number', Depth: 'Text', Repeat: 'Text', WaitingOn: 'Text', FollowUp: 'Date', HiddenUntil: 'Date', CreatedAt: 'Date', UpdatedAt: 'Date', CompletedAt: 'Date', Archived: 'Checkbox', RelationKind: 'Select',
  ProjectStatus: 'Select', TargetDate: 'Date', Progress: 'Number', ProgressChart: 'Number', IssueCount: 'Number',
};
export const fixedKind = (f: FixedField) => fixedKinds[f];

export const noValue = (op: Op) => op === 'IsEmpty' || op === 'IsNotEmpty';
export const multi = (op: Op) => op === 'In' || op === 'NotIn';
const complete = (c: Chip) => noValue(c.op) || (multi(c.op) ? (c.value as Value[]).length > 0 : c.value !== null && c.value !== '');

/** The saved filter: the untouched `advanced` part (nested Or/Not the chips cannot show) and every finished chip, all required. */
export function buildFilter(advanced: Filter | null, chips: Chip[]): Filter | null {
  const parts: Filter[] = [
    ...(advanced ? [advanced] : []),
    ...chips.filter(complete).map((c): Filter => ({ Cond: { field: c.field, op: c.op, value: noValue(c.op) ? null : c.value } })),
  ];
  return parts.length === 0 ? null : parts.length === 1 ? parts[0] : { And: parts };
}

let seq = 0;
export const newChip = (field: FieldRef, op: Op, value: Value): Chip => ({ id: ++seq, field, op, value });

/** A flat And of conditions becomes chips; anything nested is kept whole as `advanced`. */
export function splitFilter(f: Filter | null): { chips: Chip[]; advanced: Filter | null } {
  const items = !f ? [] : f.And ?? [f];
  if (items.every((x) => x.Cond)) return { chips: items.map((x) => newChip(x.Cond!.field, x.Cond!.op, x.Cond!.value)), advanced: null };
  return { chips: [], advanced: f };
}
