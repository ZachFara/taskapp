import type { FieldRef, Sort, ViewEntity } from './bindings';

export const sameField = (a: FieldRef, b: FieldRef) => a.Fixed === b.Fixed && a.Property === b.Property;

export function nextSort(current: Sort[] | null, field: FieldRef): Sort[] | null {
  const sort = current?.[0];
  if (!sort || !sameField(sort.field, field)) return [{ field, descending: false }];
  return sort.descending ? null : [{ field, descending: true }];
}

export function sortable(field: FieldRef, entity: ViewEntity = 'issue') {
  if (field.Property !== undefined) return true;
  if (entity === 'project') return ['Title', 'Status', 'StatusCategory', 'Priority', 'StartDate', 'TargetDate', 'CreatedAt', 'UpdatedAt', 'Progress', 'IssueCount', 'Archived'].includes(field.Fixed);
  return !['Label', 'RelationKind', 'TargetDate', 'Progress', 'ProgressChart', 'IssueCount'].includes(field.Fixed);
}
