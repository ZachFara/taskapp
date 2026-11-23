import type { FixedField } from './bindings';

/** The built-in issue fields a workspace can switch off, in their default order (status and title are always on). */
export const FIELDS = ['priority', 'project', 'milestone', 'cycle', 'labels', 'start', 'due', 'estimate', 'points', 'parent'] as const;
export type FieldName = (typeof FIELDS)[number];
export type IssueField = { field: FieldName; on: boolean };
export const FIELD_LABELS: Record<FieldName, string> = {
  priority: 'Priority', project: 'Project', milestone: 'Milestone', cycle: 'Cycle', labels: 'Labels', start: 'Start',
  due: 'Due', estimate: 'Estimate', points: 'Points', parent: 'Parent',
};
/** The view field each setting name stands for. */
export const FIXED: Partial<Record<FixedField, FieldName>> = {
  Priority: 'priority', Project: 'project', Milestone: 'milestone', Cycle: 'cycle', Label: 'labels', StartDate: 'start',
  DueDate: 'due', Estimate: 'estimate', Points: 'points', Parent: 'parent',
};

/** The stored list made whole: known names only, each once, anything missing on and last. */
export function normalize(raw: unknown): IssueField[] {
  const seen = new Set<FieldName>();
  const out: IssueField[] = [];
  for (const i of Array.isArray(raw) ? raw : []) {
    const f = (i as IssueField | null)?.field;
    if (f && FIELDS.includes(f) && !seen.has(f)) (seen.add(f), out.push({ field: f, on: i.on !== false }));
  }
  return [...out, ...FIELDS.filter((f) => !seen.has(f)).map((field) => ({ field, on: true }))];
}

