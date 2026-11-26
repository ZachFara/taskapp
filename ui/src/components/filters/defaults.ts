import type { FieldRef, Filter, NewIssue, Project, PropertyDef, Status, Value } from '../../lib/bindings';

export type IssueDefaults = Omit<NewIssue, 'title' | 'created_at' | 'completed_at'>;

export type FilterDefaultContext = {
  statuses: Status[];
  projects?: Project[];
  propertyDefs?: PropertyDef[];
  defaultStatusId?: number | null;
};

const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null);
const firstNum = (v: unknown): number | null => (Array.isArray(v) ? num(v[0]) : null);
const numList = (v: unknown): number[] => (Array.isArray(v) ? v.map(num).filter((x): x is number => x !== null) : []);

/**
 * Extracts concrete creation defaults from a view's filter so an issue created inside
 * that view inherits its status and matching fields instead of vanishing from the view.
 */
export function defaultsFromFilter(filter: Filter | null, ctx: FilterDefaultContext): IssueDefaults {
  if (!filter) return {};
  const out: IssueDefaults = {};
  const excludedStatuses = new Set<number>();
  const excludedCategories = new Set<string>();
  const allowedStatusSets: Set<number>[] = [];
  const allowedCategorySets: Set<string>[] = [];
  const orderedStatuses = [...ctx.statuses].sort((a, b) => a.position - b.position);
  const defaultStatus =
    orderedStatuses.find((s) => s.id === ctx.defaultStatusId) ?? orderedStatuses[0] ?? null;

  const addLabels = (ids: number[]) => {
    if (!ids.length) return;
    out.label_ids = [...new Set([...(out.label_ids ?? []), ...ids])];
  };

  const setProp = (key: string, value: Value) => {
    out.properties = { ...(out.properties ?? {}), [key]: value };
  };

  const visit = (f: Filter) => {
    if (f.And) {
      for (const child of f.And) visit(child);
      return;
    }
    if (f.Or) {
      if (f.Or.length === 1) visit(f.Or[0]);
      return;
    }
    if (f.Not || !f.Cond) return;
    const { field, op, value } = f.Cond;
    if (field.Property !== undefined) {
      applyProperty(field.Property, op, value);
      return;
    }
    applyFixed(field.Fixed!, op, value);
  };

  const applyProperty = (key: string, op: string, value: Value) => {
    const def = ctx.propertyDefs?.find((d) => d.key === key);
    if (op === 'Eq' && value !== null) {
      setProp(key, value);
    } else if (op === 'In' && Array.isArray(value) && value.length > 0) {
      setProp(key, def?.kind === 'MultiSelect' ? [value[0]] : value[0]);
    } else if (op === 'Contains' && def?.kind === 'MultiSelect' && value !== null) {
      const prev = Array.isArray(out.properties?.[key]) ? (out.properties![key] as Value[]) : [];
      if (!prev.includes(value)) setProp(key, [...prev, value]);
    }
  };

  const applyFixed = (field: NonNullable<FieldRef['Fixed']>, op: string, value: Value) => {
    switch (field) {
      case 'Status': {
        if (op === 'Eq') {
          const id = num(value);
          if (id !== null) allowedStatusSets.push(new Set([id]));
        } else if (op === 'In') {
          allowedStatusSets.push(new Set(numList(value)));
        } else if (op === 'Neq') {
          const id = num(value);
          if (id !== null) excludedStatuses.add(id);
        } else if (op === 'NotIn') {
          for (const id of numList(value)) excludedStatuses.add(id);
        }
        break;
      }
      case 'StatusCategory': {
        const toCats = (v: Value): string[] =>
          (Array.isArray(v) ? v : [v]).filter((x): x is string => typeof x === 'string').map((s) => s.toLowerCase());
        if (op === 'Eq' || op === 'In') {
          const cats = toCats(value);
          allowedCategorySets.push(new Set(cats));
        } else if (op === 'Neq' || op === 'NotIn') {
          for (const c of toCats(value)) excludedCategories.add(c);
        }
        break;
      }
      case 'Project': {
        const id = op === 'Eq' ? num(value) : op === 'In' ? firstNum(value) : null;
        if (id !== null && (!ctx.projects?.length || ctx.projects.some((p) => p.id === id))) out.project_id = id;
        break;
      }
      case 'Priority': {
        const p = op === 'Eq' ? num(value) : op === 'In' ? firstNum(value) : null;
        if (p !== null && p >= 0 && p <= 4) out.priority = p;
        break;
      }
      case 'Cycle': {
        const id = op === 'Eq' ? num(value) : op === 'In' ? firstNum(value) : null;
        if (id !== null) out.cycle_id = id;
        break;
      }
      case 'Milestone': {
        const id = op === 'Eq' ? num(value) : op === 'In' ? firstNum(value) : null;
        if (id !== null) out.milestone_id = id;
        break;
      }
      case 'Parent': {
        const id = op === 'Eq' ? num(value) : op === 'In' ? firstNum(value) : null;
        if (id !== null) out.parent_id = id;
        break;
      }
      case 'Label': {
        if (op === 'Eq' || op === 'Contains') {
          const id = num(value);
          if (id !== null) addLabels([id]);
        } else if (op === 'In') {
          const id = firstNum(value);
          if (id !== null) addLabels([id]);
        }
        break;
      }
      case 'StartDate': {
        if (op === 'Eq' && typeof value === 'string' && value) out.start_date = value;
        break;
      }
      case 'DueDate': {
        if (op === 'Eq' && typeof value === 'string' && value) out.due = { Date: value };
        break;
      }
      case 'Estimate': {
        const mins = op === 'Eq' ? num(value) : null;
        if (mins !== null) out.estimate_minutes = mins;
        break;
      }
      case 'Points': {
        const pts = op === 'Eq' ? num(value) : null;
        if (pts !== null) out.points = pts;
        break;
      }
    }
  };

  visit(filter);

  if ((allowedStatusSets.length || allowedCategorySets.length || excludedStatuses.size || excludedCategories.size) && defaultStatus) {
    const allowed = (s: Status) =>
      !excludedStatuses.has(s.id) && !excludedCategories.has(s.category.toLowerCase()) &&
      allowedStatusSets.every((set) => set.has(s.id)) &&
      allowedCategorySets.every((set) => set.has(s.category.toLowerCase()));
    const preferred = allowedStatusSets[0]
      ? [...allowedStatusSets[0]].flatMap((id) => orderedStatuses.find((s) => s.id === id) ?? [])
      : orderedStatuses;
    const chosen = allowed(defaultStatus) ? defaultStatus : preferred.find(allowed);
    if (chosen && (allowedStatusSets.length || allowedCategorySets.length || chosen.id !== defaultStatus.id)) out.status_id = chosen.id;
  }

  return out;
}

/** Merges two issue default bags; `override` wins on scalar fields while labels and properties combine. */
export function mergeIssueDefaults(
  base: IssueDefaults | null | undefined,
  override: IssueDefaults | null | undefined,
): IssueDefaults {
  const out: IssueDefaults = { ...(base ?? {}), ...(override ?? {}) };
  if (override?.label_ids && override.label_ids.length === 0) {
    out.label_ids = [];
  } else if (base?.label_ids?.length || override?.label_ids?.length) {
    const merged = [...new Set([...(base?.label_ids ?? []), ...(override?.label_ids ?? [])])];
    if (merged.length) out.label_ids = merged;
  }
  if (base?.properties || override?.properties) {
    const props = { ...(base?.properties ?? {}), ...(override?.properties ?? {}) };
    for (const [key, value] of Object.entries(props)) {
      if (value === null) delete props[key];
    }
    if (Object.keys(props).length) out.properties = props;
    else delete out.properties;
  }
  return out;
}
