import type { Api } from './api';
import { hexOf, iconOk, keyFor } from './api.mock.palette';
import { planCsv, projectSlug } from './api.mock.import';
import { FIELDS } from './issueFields';
import { parseQuickadd } from './api.mock.quickadd';
import * as insights from './api.mock.insights';
import { forecast, metrics } from './api.mock.metrics';
import { stepLabel } from './api.mock.stepLabel';
import { byStatusOrder, flowDefaults } from './statusOrder';
import { builtInKinds, relationKindsApi } from './api.mock.relationKinds';
import { decisionsApi } from './api.mock.decisions';
import { attachmentsApi } from './api.mock.attachments';
import { foldersApi } from './api.mock.folders';
import { scrubDef, seedViewTemplates, withDisplay } from './api.mock.views';
import type {
  AppliedTemplate, Bar, Change, Measure, ProgressScope, Comment, Cycle, Decision, Event, EventInput, FieldRef, Filter, Issue, IssueTemplate, Label, Layout, Milestone,
  NewIssue, Pin, Placement, Project, ProjectRow, ProjectStatus, ProjectStatusCategory, PropertyAppliesTo, PropertyDef, Relation, Reschedule, SavedView, Status, FixedField, TemplateFields,
  TemplateInput, Value, ViewDef, ViewEntity, ViewFolder, ViewTemplate, WorkBlock,
} from './bindings';

export type MockSeed = {
  statuses: Status[];
  projects: Project[];
  projectStatuses?: ProjectStatus[];
  propertyDefs: PropertyDef[];
  issues: Issue[];
  labels?: Label[];
  cycles?: Cycle[];
  decisions?: Decision[];
  views?: SavedView[];
  pins?: Pin[];
  milestones?: Milestone[];
  relations?: Relation[];
  templates?: IssueTemplate[];
  viewTemplates?: ViewTemplate[];
  viewFolders?: ViewFolder[];
  workspace?: Workspace;
  /** Changes that predate the mock, for progress charts (the real seed writes the same rows). */
  history?: Change[];
};

export type Workspace = { setup_done: boolean; name: string | null; key: string; timezone: string };
/** The workspace commands are built on the backend in parallel and are not in the generated bindings yet. */
export type MockApi = Api & {
  workspace: { status: () => Promise<Workspace>; setup: (name: string, key: string, timezone: string | null) => Promise<Workspace> };
};

const nullableIssueFields = [
  'project_id', 'milestone_id', 'parent_id', 'cycle_id', 'start_date', 'due', 'estimate_minutes', 'points',
] as const;
// The backend's error text is `<kind>: <detail>`; the UI shows it as is.
const fail = (kind: string, message: string): never => {
  throw { kind, message: `${{ NotFound: 'not found', Db: 'database error' }[kind] ?? kind.toLowerCase()}: ${message}` };
};
const addDays = (date: string, n: number) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + n);
  return d.toISOString().slice(0, 10);
};
// A timed due date counts as its day in its own zone, like the backend's due_date column.
const dueDay = (i: Issue) => {
  if (i.due?.Date) return i.due.Date;
  if (!i.due?.DateTime) return null;
  const { utc, tz } = i.due.DateTime;
  try {
    return new Date(utc).toLocaleDateString('sv', { timeZone: tz });
  } catch {
    return utc.slice(0, 10);
  }
};
const nothing = (_?: unknown) => null;
/** Same rule as core: 0 to 1000 with exactly the precision of one decimal (no rounding of float noise). */
const validPoints = (p: number) => Number.isFinite(p) && p >= 0 && p <= 1000 && Number(p.toFixed(1)) === p;
const dayDiff = (a: string, b: string) => (Date.parse(b) - Date.parse(a)) / 86_400_000;

const isDate = (v: unknown) => typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v) && !Number.isNaN(Date.parse(v)) && new Date(v).toISOString().slice(0, 10) === v;
const isUrl = (v: unknown) => typeof v === 'string' && /^https?:\/\/\S+$/i.test(v);
/** Same rule as the backend: a value must fit its definition's kind. */
function validValue(d: PropertyDef, v: Value): boolean {
  const has = (x: Value) => typeof x === 'string' && d.options.some((o) => o.value === x);
  switch (d.kind) {
    case 'Text': return typeof v === 'string';
    case 'Number': return typeof v === 'number';
    case 'Date': return isDate(v);
    case 'Select': return has(v);
    case 'MultiSelect': return Array.isArray(v) && v.every(has) && new Set(v).size === v.length;
    case 'Checkbox': return typeof v === 'boolean';
    case 'Url': return isUrl(v);
    case 'NoteLink': return typeof v === 'string' && (v.startsWith('obsidian://') || /\.md$/i.test(v));
  }
}

const fieldsOf = (f: Partial<TemplateFields> = {}): TemplateFields => ({
  description: null, status_id: null, priority: null, project_id: null, label_ids: [], estimate_minutes: null, points: null, properties: {}, ...f,
});

export function createMockApi(seed: MockSeed = fixtures, tz = Intl.DateTimeFormat().resolvedOptions().timeZone): MockApi {
  const s = structuredClone(seed);
  let workspace: Workspace = s.workspace ?? { setup_done: false, name: null, key: 'ABC', timezone: tz };
  const labels = s.labels ?? [];
  const projectStatuses = (s.projectStatuses ??= structuredClone(projectStatusSeed));
  const cycles = s.cycles ?? [];
  const events: Event[] = [];
  const decisions: Decision[] = s.decisions ?? [];
  const views = s.views ?? [];
  const viewTemplates: ViewTemplate[] = s.viewTemplates ?? [];
  const folders: ViewFolder[] = s.viewFolders ?? [];
  // shortcut: a deleted item's placement stays until its id is reused, when it would show in the old folder; fixtures never reuse ids.
  const placed: Placement[] = [];
  const milestones: Milestone[] = s.milestones ?? [];
  const comments: Comment[] = [];
  const relations: Relation[] = s.relations ?? [];
  const relationKinds = builtInKinds();
  const kinds = relationKindsApi(relationKinds, relations, fail);
  const pins: Pin[] = s.pins ?? [];
  const blocks: WorkBlock[] = [];
  const changes: Change[] = [...(s.history ?? [])];
  // One step per command that changed anything, like the backend; undo and redo cover every entity.
  type Step = { label: string; before: Issue[]; after: Issue[]; rows: Change[]; beforeRest: string; afterRest: string };
  const undoSteps: Step[] = [];
  const redoSteps: Step[] = [];
  const collections = () => [s.statuses, s.projects, projectStatuses, s.propertyDefs, labels, cycles, events, decisions, templates, relations, relationKinds, comments, views, milestones, pins, viewTemplates, folders, placed, importLog] as unknown[][];
  const others = () => JSON.stringify(collections());
  const status = () => ({ undo: undoSteps.at(-1)?.label ?? null, redo: redoSteps.at(-1)?.label ?? null });
  // A command that calls other commands (bulk update) is one step: only the outermost call records it.
  let depth = 0;
  const tracked = async <R>(label: (r: R) => string, run: () => Promise<R>): Promise<R> => {
    if (depth) return run();
    const [before, rest, mark] = [structuredClone(s.issues), others(), changes.length];
    depth++;
    const out = await run().finally(() => depth--);
    const after = structuredClone(s.issues);
    const touchedIssues = JSON.stringify(before) !== JSON.stringify(after);
    if (touchedIssues || rest !== others()) {
      undoSteps.push({ label: label(out), before, after, rows: changes.slice(mark), beforeRest: rest, afterRest: others() });
      redoSteps.length = 0;
    }
    return out;
  };
  const undoRedo = (undo: boolean) => {
    const from = undo ? undoSteps : redoSteps;
    const st = from.at(-1) ?? fail('Invalid', `nothing to ${undo ? 'undo' : 'redo'}`);
    from.pop();
    const target = JSON.parse(undo ? st.beforeRest : st.afterRest) as unknown[][];
    collections().forEach((rows, k) => {
      if (JSON.stringify(rows) !== JSON.stringify(target[k])) rows.splice(0, rows.length, ...target[k]);
    });
    for (const kept of keptStatuses) if (!s.statuses.some((x) => x.id === kept.id)) s.statuses.push(kept);
    const [from_, to] = undo ? [st.after, st.before] : [st.before, st.after];
    for (const was of from_) {
      const target = to.find((x) => x.id === was.id);
      const at = s.issues.findIndex((x) => x.id === was.id);
      // A created issue is undone by archiving it and redone by restoring it.
      const next = target ? structuredClone(target) : { ...was, archived_at: undo ? now() : null };
      if (target && JSON.stringify(target) === JSON.stringify(was)) continue;
      next.updated_at = now();
      if (at >= 0) s.issues[at] = next;
    }
    // Redo brings back an issue the step created: it is only in the later state, so the loop above never met it.
    for (const t of to) {
      const at = s.issues.findIndex((x) => x.id === t.id);
      if (!from_.some((x) => x.id === t.id) && at >= 0) s.issues[at] = { ...structuredClone(t), updated_at: now() };
    }
    // Undo replays the step's rows newest first with old and new swapped; a create row pair becomes an archive.
    const src = undo ? 'Undo' : 'Redo';
    for (const r of undo ? [...st.rows].reverse() : st.rows) {
      const id = Number(r.entity_id);
      if (r.field === 'title' && r.old_value === null) continue;
      if (r.field === 'key') rec(id, 'archived_at', undo ? null : now(), undo ? now() : null, src);
      else if (r.field.startsWith('properties.')) rec(id, 'properties', getIssue(id).properties, getIssue(id).properties, src);
      else rec(id, r.field, undo ? r.new_value : r.old_value, undo ? r.old_value : r.new_value, src);
    }
    (undo ? redoSteps : undoSteps).push(st);
    return status();
  };
  const settings = new Map<string, Value>([['issue_prefix', workspace.key], ['estimation', 'both'], ['point_scale', 'fibonacci']]);
  // The same checks and words as the real backend (crates/core/src/settings.rs), so the contract cases agree.
  const checkIssueFields = (v: unknown) => {
    const bad = (why: string) => fail('Invalid', `setting issue_fields: ${why}`);
    if (!Array.isArray(v)) return bad('must be a list of {field, on}');
    const seen = new Set<string>();
    for (const i of v) {
      if (typeof i?.field !== 'string' || typeof i.on !== 'boolean') return bad('each entry needs a field name and a true or false `on`');
      if (!(FIELDS as readonly string[]).includes(i.field) || seen.has(i.field)) return bad(`unknown or repeated field \`${i.field}\``);
      seen.add(i.field);
    }
  };
  const choices: Record<string, string[]> = { estimation: ['none', 'points', 'time', 'both'], point_scale: ['fibonacci', 'linear', 'powers', 'tshirt'] };
  const now = () => new Date().toISOString();
  // Issue history only, in the order the backend records it; other entities' history is not shown by any screen.
  const rec = (issue: number, field: string, old: Value, value: Value, source: Change['source'] = 'Ui') =>
    void changes.push({ id: next(changes), at: now(), source, entity: 'issue', entity_id: String(issue), field, old_value: old, new_value: value });
  const checkPin = (kind: Pin['kind'], target: string) => {
    const lower = target.toLowerCase();
    const valid = kind === 'Url' ? /^(https?|obsidian):\/\//.test(lower) : kind === 'Note' ? lower.startsWith('obsidian://') : target.trim() !== '';
    if (!valid) fail('Invalid', `pin target ${JSON.stringify(target)} for ${kind.toLowerCase()}`);
  };
  const next = (rows: { id: number }[]) => Math.max(0, ...rows.map((r) => r.id)) + 1;
  // The real seed adds these through the core, which records them.
  for (const r of relations) rec(r.from, 'relation', null, r);
  const find = <T extends { id: number }>(rows: T[], id: number, what: string) =>
    rows.find((r) => r.id === id) ?? fail('NotFound', `${what} ${id}`);
  const remove = <T extends { id: number }>(rows: T[], id: number, what: string) =>
    void rows.splice(rows.indexOf(find(rows, id, what)), 1);
  const category = (i: Issue) => s.statuses.find((st) => st.id === i.status_id)?.category;

  const projectStatusOf = (p: Project) => projectStatuses.find((x) => x.id === p.status_id);
  const progressOf = (id: number): [number, number] => {
    const rows = s.issues.filter((i) => i.project_id === id && !i.archived_at);
    return [rows.filter((i) => category(i) === 'Completed').length, rows.length];
  };
  // What a project view can filter and sort by (core: views::project_col).
  function projectValue(p: Project, f: FieldRef): Value {
    if (f.Property !== undefined) {
      if (!s.propertyDefs.some((d) => d.key === f.Property && d.applies_to !== 'Issues')) fail('Invalid', 'unknown property');
      return p.properties[f.Property] ?? null;
    }
    const [done, total] = progressOf(p.id);
    const by: Partial<Record<FixedField, Value>> = {
      Title: p.name, Status: p.status_id, StatusCategory: projectStatusOf(p)?.category.toLowerCase() ?? null, Priority: p.priority,
      Label: p.label_ids, StartDate: p.start_date, TargetDate: p.target_date, Archived: p.archived_at !== null, CreatedAt: p.created_at, UpdatedAt: p.updated_at,
      IssueCount: total, Progress: total ? (100 * done) / total : 0, ProgressChart: total ? (100 * done) / total : 0,
    };
    if (!(f.Fixed! in by)) fail('Invalid', 'not a project field');
    return by[f.Fixed!]!;
  }

  function fieldValue(i: Issue, f: FieldRef): Value {
    const prop = f.Property;
    if (prop !== undefined) {
      if (!s.propertyDefs.some((d) => d.key === prop)) fail('Invalid', 'unknown property');
      return i.properties[prop] ?? null;
    }
    if (f.Fixed === 'TargetDate' || f.Fixed === 'Progress' || f.Fixed === 'ProgressChart' || f.Fixed === 'IssueCount') fail('Invalid', 'a project field, not an issue field');
    const by: Record<Exclude<FixedField, 'TargetDate' | 'Progress' | 'ProgressChart' | 'IssueCount'>, Value> = {
      ProjectStatus: s.projects.find((x) => x.id === i.project_id)?.status_id ?? null,
      Title: i.title, Status: i.status_id, StatusCategory: category(i)?.toLowerCase() ?? null, Priority: i.priority,
      Project: i.project_id, Milestone: i.milestone_id, Cycle: i.cycle_id, Label: i.label_ids,
      Parent: i.parent_id, StartDate: i.start_date, DueDate: dueDay(i), Estimate: i.estimate_minutes, Points: i.points,
      CreatedAt: i.created_at, UpdatedAt: i.updated_at, CompletedAt: i.completed_at,
      Archived: i.archived_at !== null, Key: i.id,
      SubIssueProgress: s.issues.filter((c) => c.parent_id === i.id && !c.archived_at).length,
      RelationCount: relations.filter((r) => r.from === i.id || r.to === i.id).length,
      RelationKind: relations.filter((r) => r.from === i.id || r.to === i.id).map((r) => r.kind),
      CommentCount: comments.filter((c) => c.issue_id === i.id).length,
    };
    return by[f.Fixed as keyof typeof by];
  }

  const isEmpty = (v: Value) => v === null || v === '' || (Array.isArray(v) && v.length === 0);
  const eq = (have: Value, want: Value) =>
    Array.isArray(have) && !Array.isArray(want) ? have.includes(want) : have === want;
  const holds = <T,>(i: T, f: Filter, get: (r: T, f: FieldRef) => Value): boolean => {
    if (f.And) return f.And.every((x) => holds(i, x, get));
    if (f.Or) return f.Or.some((x) => holds(i, x, get));
    if (f.Not) return !holds(i, f.Not, get);
    const { field, op, value: want } = f.Cond!;
    const have = get(i, field);
    const list = Array.isArray(want) ? want : [want];
    const cmp = (test: (a: number | string, b: number | string) => boolean) =>
      have !== null && want !== null && typeof have === typeof want && test(have as string, want as string);
    switch (op) {
      case 'Eq': return eq(have, want);
      case 'Neq': return !eq(have, want);
      case 'In': return list.some((w) => eq(have, w));
      case 'NotIn': return !list.some((w) => eq(have, w));
      case 'Contains':
        return typeof have === 'string'
          ? have.toLowerCase().includes(String(want).toLowerCase())
          : Array.isArray(have) && have.includes(want);
      case 'Lt': return cmp((a, b) => a < b);
      case 'Lte': return cmp((a, b) => a <= b);
      case 'Gt': return cmp((a, b) => a > b);
      case 'Gte': return cmp((a, b) => a >= b);
      case 'IsEmpty': return isEmpty(have);
      case 'IsNotEmpty': return !isEmpty(have);
    }
  };
  // `today`, `today+7` or `today-14` on a date field names a day counted from today (core: views::relative_day).
  const relativeDay = (v: Value, today: string): Value => {
    if (typeof v !== 'string' || !v.startsWith('today')) return v;
    const m = /^today(?:([+-])(\d+))?$/.exec(v);
    if (!m) fail('Invalid', `bad relative date ${v}`);
    const d = new Date(`${today}T00:00:00Z`);
    d.setUTCDate(d.getUTCDate() + (m![1] ? Number(`${m![1]}${m![2]}`) : 0));
    return d.toISOString().slice(0, 10);
  };
  const dateFixed = ['StartDate', 'DueDate', 'TargetDate', 'CreatedAt', 'UpdatedAt', 'CompletedAt'];
  const withRelativeDays = (f: Filter, defs: PropertyDef[], today: string): Filter => {
    if (f.And) return { And: f.And.map((x) => withRelativeDays(x, defs, today)) };
    if (f.Or) return { Or: f.Or.map((x) => withRelativeDays(x, defs, today)) };
    if (f.Not) return { Not: withRelativeDays(f.Not, defs, today) };
    const { field, op, value } = f.Cond!;
    const isDate = field.Fixed !== undefined ? dateFixed.includes(field.Fixed) : defs.some((d) => d.key === field.Property && d.kind === 'Date');
    return { Cond: { field, op, value: isDate ? relativeDay(value, today) : value } };
  };
  // Filter, then sort (nulls last in either direction), like core's compiled query; `get` reads a field of a row.
  function arrange<T extends { id: number; archived_at: string | null }>(all: T[], stored: ViewDef, defs: PropertyDef[], get: (r: T, f: FieldRef) => Value) {
    // A reference to a property that no longer exists is dropped, like core does.
    const def = scrubDef(stored, (k) => !defs.some((d) => d.key === k));
    const filter = def.filter && withRelativeDays(def.filter, defs, zoneToday());
    const rows = all.filter((i) => (def.include_archived || !i.archived_at) && (!filter || holds(i, filter, get)));
    for (const { field, descending } of [...def.sorts].reverse()) {
      rows.sort((a, b) => {
        const [x, y] = [get(a, field), get(b, field)];
        if (x === y) return 0;
        if (x === null) return 1;
        if (y === null) return -1;
        return (x < y ? -1 : 1) * (descending ? -1 : 1);
      });
    }
    return rows;
  }
  const query = (stored: ViewDef) => arrange(s.issues, stored, s.propertyDefs, fieldValue);
  const queryProjects = (stored: ViewDef): ProjectRow[] =>
    arrange(s.projects, stored, s.propertyDefs.filter((d) => d.applies_to !== 'Issues'), projectValue).map((project) => {
      const [issue_done, issue_total] = progressOf(project.id);
      return { project, issue_total, issue_done };
    });

  // A new view lists `entity`; an existing one keeps what it lists.
  const saveView = async (id: number | null, name: string, d: ViewDef, icon: string | null, color: string | null, entity: ViewEntity) => {
    checkIcon(icon);
    const key = colorKey(color);
    if (id === null) {
      const v = { id: next(views), name, entity, def: withDisplay(d), position: Math.max(-1, ...views.map((x) => x.position)) + 1, icon: icon ?? null, color: key, template_id: null, folder_id: null };
      views.push(v);
      return v;
    }
    const v = find(views, id, 'view');
    return Object.assign(v, { name, def: withDisplay(d), icon: icon ?? v.icon, color: key ?? v.color });
  };
  const viewIssues = (id: number) => query(find(views, id, 'view').def);
  const checkIssue = (i: Issue, props: Record<string, Value>) => {
    if (!i.title.trim()) fail('Invalid', 'title is empty');
    if (i.priority > 4) fail('Invalid', `priority ${i.priority} is not 0 to 4`);
    if (i.points != null && !validPoints(i.points)) fail('Invalid', `points ${i.points} must be 0 to 1000 with at most one decimal`);
    if (!s.statuses.some((x) => x.id === i.status_id)) fail('Invalid', `status ${i.status_id} does not exist`);
    for (const [k, v] of Object.entries(props)) {
      const d = s.propertyDefs.find((x) => x.key === k);
      if (d && !validValue(d, v)) fail('Invalid', `${JSON.stringify(v)} is not a valid ${d.kind} for ${k}`);
    }
  };
  const done = (i: Issue) => ['Completed', 'Canceled'].includes(category(i) ?? '');
  const create = (input: Parameters<Api['issues']['create']>[0]): Issue => {
    const number = Math.max(0, ...s.issues.map((i) => i.id)) + 1;
    const at = now();
    const issue: Issue = {
      id: number, key: `${settings.get('issue_prefix')}-${number}`, title: (input.title ?? '').trim(),
      description: input.description ?? '',
      // The chosen starting status, else the first (also when the chosen one was deleted), as the backend.
      status_id: input.status_id ?? (s.statuses.find((st) => st.id === settings.get('default_status')) ?? [...s.statuses].sort((a, b) => a.position - b.position)[0]).id,
      priority: input.priority ?? 0, project_id: input.project_id ?? null, milestone_id: input.milestone_id ?? null,
      parent_id: input.parent_id ?? null, cycle_id: input.cycle_id ?? null, label_ids: [...new Set(input.label_ids ?? [])].sort((a, b) => a - b),
      start_date: input.start_date ?? null, due: input.due ?? null, estimate_minutes: input.estimate_minutes ?? null, points: input.points ?? null,
      properties: input.properties ?? {}, created_at: input.created_at ?? at, updated_at: at,
      completed_at: input.completed_at ?? null, archived_at: null,
    };
    checkIssue(issue, issue.properties);
    issue.completed_at ??= done(issue) ? at : null;
    s.issues.push(issue);
    rec(issue.id, 'title', null, issue.title);
    rec(issue.id, 'key', null, issue.key);
    return issue;
  };
  const getIssue = (id: number) => find(s.issues, id, 'issue');
  const templates: IssueTemplate[] = s.templates ?? [];
  // Same rule as core: a reference that no longer resolves is dropped and named in `skipped`.
  const liveRefs = (f: Partial<TemplateFields>, skipped: string[]): TemplateFields => {
    const status = s.statuses.some((x) => x.id === f.status_id) ? f.status_id : null;
    if (f.status_id != null && status === null) skipped.push(`status ${f.status_id} does not exist`);
    const project = s.projects.some((x) => x.id === f.project_id) ? f.project_id : null;
    if (f.project_id != null && project === null) skipped.push(`project ${f.project_id} does not exist`);
    const label_ids = (f.label_ids ?? []).filter((l) => {
      const ok = labels.some((x) => x.id === l);
      if (!ok) skipped.push(`label ${l} does not exist`);
      return ok;
    });
    return { ...f, status_id: status, project_id: project, label_ids, properties: f.properties ?? {} };
  };
  const applyTemplate = (t: IssueTemplate, o: NewIssue): AppliedTemplate => {
    const skipped: string[] = [];
    const f = liveRefs(t.body.fields ?? {}, skipped);
    const own = o.title?.trim() ?? '';
    const prefix = t.body.title_prefix ?? '';
    const title = own ? `${prefix}${own}` : prefix.trimEnd();
    const issue = create({
      title, description: o.description ?? f.description, status_id: o.status_id ?? f.status_id,
      priority: o.priority ?? f.priority, project_id: o.project_id ?? f.project_id,
      label_ids: o.label_ids ?? f.label_ids, estimate_minutes: o.estimate_minutes ?? f.estimate_minutes, points: o.points ?? f.points,
      properties: { ...f.properties, ...o.properties }, start_date: o.start_date, due: o.due,
    });
    const sub_issues = (t.body.sub_issues ?? []).map((sub) => {
      const sf = liveRefs(sub.fields ?? {}, skipped);
      return create({
        title: sub.title, description: sf.description, status_id: sf.status_id, priority: sf.priority,
        project_id: sf.project_id, label_ids: sf.label_ids, estimate_minutes: sf.estimate_minutes, points: sf.points,
        properties: sf.properties, parent_id: issue.id,
      });
    });
    return { issue, sub_issues, skipped };
  };
  // Same checks as core: names, slug shape, icon and colour, priority range; the body is stored with every field present.
  const templateOf = (id: number, input: TemplateInput): IssueTemplate => {
    if (!input.name.trim()) fail('Invalid', 'template name is empty');
    if (!/^[a-z0-9-]+$/.test(input.slug)) fail('Invalid', `slug ${input.slug} must be lowercase letters, digits or dashes`);
    checkIcon(input.icon);
    const body = {
      title_prefix: input.body.title_prefix ?? '', fields: fieldsOf(input.body.fields),
      sub_issues: (input.body.sub_issues ?? []).map((x) => ({ title: x.title ?? '', fields: fieldsOf(x.fields) })),
    };
    if ([body.fields, ...body.sub_issues.map((x) => x.fields)].some((f) => (f.priority ?? 0) > 4)) fail('Invalid', 'template priority is not 0 to 4');
    return { id, name: input.name, slug: input.slug, icon: input.icon ?? null, color: colorKey(input.color), body };
  };
  const today = () => new Date().toLocaleDateString('sv', { timeZone: tz });
  // Fields the quick add must not read: switched off in `issue_fields`, and points when estimation has none.
  const fieldsOff = () => [
    ...((settings.get('issue_fields') as { field: string; on: boolean }[] | undefined) ?? []).filter((f) => !f.on).map((f) => f.field),
    ...(['none', 'time'].includes(String(settings.get('estimation'))) ? ['points'] : []),
  ];
  const parse = (input: string) => parseQuickadd(input, today(), tz, fieldsOff());
  // Ports of schedule.rs: a bar is start..due, or one end derived from the estimate (1 day = 480 minutes, at least 1).
  const barFor = (i: Issue): Bar | null => {
    const due = dueDay(i);
    const span = (m: number) => Math.max(1, Math.ceil(m / 480)) - 1;
    if (i.start_date && due) return { issue_id: i.id, start: i.start_date, end: due, from_estimate: false };
    if (due && i.estimate_minutes != null) return { issue_id: i.id, start: addDays(due, -span(i.estimate_minutes)), end: due, from_estimate: true };
    if (i.start_date && i.estimate_minutes != null) return { issue_id: i.id, start: i.start_date, end: addDays(i.start_date, span(i.estimate_minutes)), from_estimate: true };
    return null;
  };
  const zoneToday = () => new Date().toLocaleDateString('sv', { timeZone: workspace.timezone });
  // The issues in scope now, their timelines and the scope's default day range (like core's progress_on).
  const scoped = (scope: ProgressScope, measure: Measure) => {
    const now = zoneToday();
    let rows: Issue[];
    let own: [string, string] | null = null;
    let project: Project | undefined;
    if ('Project' in scope) {
      project = find(s.projects, scope.Project, 'project');
      rows = s.issues.filter((i) => i.project_id === scope.Project && !i.archived_at);
    } else if ('Cycle' in scope) {
      const c = find(cycles, scope.Cycle, 'cycle');
      own = [c.start_date, c.end_date];
      // Issues that left the cycle still count on the days they were in it.
      rows = s.issues.filter((i) => !i.archived_at && (i.cycle_id === scope.Cycle || changes.some((x) => x.entity === 'issue' && x.entity_id === String(i.id) && x.field === 'cycle_id' && x.old_value === scope.Cycle)));
    } else {
      rows = viewIssues(scope.View);
      own = [addDays(now, -29), now];
    }
    const tls = insights.timelines(s.issues.filter((i) => rows.includes(i)), changes, s.statuses, measure, workspace.timezone, 'Cycle' in scope ? { id: scope.Cycle, start: own![0] } : undefined);
    const [first, last] = insights.activity(tls, now);
    const from = project ? (project.start_date ?? first) : own![0];
    const range: [string, string] = project ? [from, (project.target_date ?? last) > from ? (project.target_date ?? last) : from] : own!;
    return { tls, range };
  };
  const blockedBy = () => {
    const m = new Map<number, number[]>();
    for (const r of relations) if (r.kind === 'blocks') m.set(r.from, [...(m.get(r.from) ?? []), r.to]);
    return m;
  };
  const shiftDependents = (moved: Reschedule): Reschedule[] => {
    const next = blockedBy();
    const bars = new Map(s.issues.flatMap((i) => { const b = barFor(i); return b ? [[i.id, b] as const] : []; }));
    const dates = new Map<number, [string, string]>([[moved.issue_id, [moved.start_date, moved.due_date]]]);
    const queue = [moved.issue_id];
    let budget = s.issues.length ** 2 + 1;
    for (let id = queue.shift(); id !== undefined && --budget > 0; id = queue.shift()) {
      const blockerDue = dates.get(id)![1];
      for (const dep of next.get(id) ?? []) {
        const bar = bars.get(dep);
        if (!bar) continue;
        const [start, end] = dates.get(dep) ?? [bar.start, bar.end];
        if (start > blockerDue) continue;
        const delta = dayDiff(start, blockerDue) + 1;
        dates.set(dep, [addDays(start, delta), addDays(end, delta)]);
        queue.push(dep);
      }
    }
    return [...dates].filter(([id]) => id !== moved.issue_id).sort(([a], [b]) => a - b)
      .map(([issue_id, [start_date, due_date]]) => ({ issue_id, start_date, due_date }));
  };
  const criticalPath = (rows: Issue[]): number[] => {
    const next = blockedBy();
    const bars = new Map(rows.flatMap((i) => { const b = barFor(i); return b ? [[i.id, b] as const] : []; }));
    const memo = new Map<number, [number, number[]]>();
    const longest = (id: number, onPath: number[]): [number, number[]] => {
      const hit = memo.get(id);
      if (hit) return hit;
      onPath.push(id);
      let best: [number, number[]] = [0, []];
      for (const n of next.get(id) ?? []) {
        if (!bars.has(n) || onPath.includes(n)) continue;
        const cand = longest(n, onPath);
        if (cand[0] > best[0] || (cand[0] === best[0] && (cand[1][0] ?? -Infinity) < (best[1][0] ?? -Infinity))) best = cand;
      }
      onPath.pop();
      const b = bars.get(id)!;
      const res: [number, number[]] = [best[0] + dayDiff(b.start, b.end) + 1, [id, ...best[1]]];
      memo.set(id, res);
      return res;
    };
    let best: [number, number[]] = [0, []];
    for (const id of [...bars.keys()].sort((a, b) => a - b)) {
      const cand = longest(id, []);
      if (cand[0] > best[0]) best = cand;
    }
    return best[1];
  };
  const upsert = <T extends { id: number }>(rows: T[], row: T, what: string) =>
    Object.assign(find(rows, row.id, what), row);
  const projectDefaults = () => ({ status_id: [...projectStatuses].sort((a, b) => a.position - b.position)[0].id, priority: 0, label_ids: [] as number[], properties: {} as Record<string, Value> });
  const def = (key: string) => s.propertyDefs.find((d) => d.key === key) ?? fail('NotFound', `property ${key}`);
  const backups: string[] = [];
  const importLog: { external_id: string; issue_id: number }[] = [];
  // Statuses an import created: the backend does not record them, so undo leaves them (archived issues still point at them).
  const keptStatuses: Status[] = [];
  const ok = async () => null;
  // Sets position to each row's index in `order`; like the real commands, the order must list every row once.
  const reposition = <T extends { position: number }, K>(rows: T[], order: K[], keyOf: (r: T) => K, what: string) => {
    if (order.length !== rows.length || rows.some((r) => order.filter((k) => k === keyOf(r)).length !== 1))
      fail('Invalid', `reorder must list every ${what} exactly once`);
    for (const r of rows) r.position = order.indexOf(keyOf(r));
  };
  // Same checks as core's palette: an icon is `icon:<id>` or `emoji:<emoji>`, a colour a palette key or a hex near one.
  const checkIcon = (icon?: string | null) => icon == null || iconOk(icon) || fail('Invalid', `icon ${icon} must be icon:<id> or emoji:<emoji>`);
  const colorKey = (color?: string | null) => (color == null ? null : (keyFor(color) ?? fail('Invalid', `color ${color} is not a palette key`)));
  // Times are local wall-clock text; the backend always writes seconds.
  const clock = (t: string) => (t.length === 16 ? `${t}:00` : t);
  const eventOf = (id: number, i: EventInput, issue_ids: number[]): Event => {
    const [starts_at, ends_at] = [clock(i.starts_at), i.ends_at === null ? null : clock(i.ends_at)];
    if (!i.title.trim()) fail('Invalid', 'an event needs a title');
    if (ends_at !== null && ends_at < starts_at) fail('Invalid', 'an event cannot end before it starts');
    checkIcon(i.icon);
    return { id, ...i, starts_at, ends_at, color: colorKey(i.color), issue_ids };
  };
  const off = { reminders: 'Off', gcal: 'Off', export: 'Off', backup: 'Off' } as const;

  const api: MockApi = {
    issues: {
      create: async (input) => create(input),
      update: async (id, patch) => {
        const i = getIssue(id);
        const old = structuredClone(i);
        for (const [k, v] of Object.entries(patch) as [keyof Issue & string, never][]) {
          if (v === undefined || k === 'properties') continue;
          if (v === null && !(nullableIssueFields as readonly string[]).includes(k)) continue;
          (i as Record<string, unknown>)[k] = k === 'label_ids' ? [...new Set(v as number[])].sort((a, b) => a - b) : v;
        }
        for (const [k, v] of Object.entries(patch.properties ?? {})) {
          if (v === null) delete i.properties[k];
          else i.properties[k] = v;
        }
        const changed = (Object.keys(i) as (keyof Issue)[]).filter((k) => JSON.stringify(i[k]) !== JSON.stringify(old[k]));
        if (!changed.length) return old;
        try {
          checkIssue(i, Object.fromEntries(Object.entries(i.properties).filter(([k, v]) => JSON.stringify(old.properties[k]) !== JSON.stringify(v))));
          for (let p = i.parent_id, hops = 0; p !== null && hops < 10_000; hops++, p = getIssue(p).parent_id) {
            if (p === i.id) fail('Invalid', 'parent would create a cycle');
          }
        } catch (e) {
          Object.assign(i, old);
          throw e;
        }
        const order = ['title', 'description', 'status_id', 'priority', 'project_id', 'milestone_id', 'parent_id', 'cycle_id', 'label_ids', 'start_date', 'due', 'estimate_minutes', 'points', 'properties'] as const;
        for (const k of order) if (changed.includes(k)) rec(id, k, old[k] as Value, i[k] as Value);
        i.updated_at = now();
        if (i.status_id !== old.status_id) i.completed_at = done(i) ? (done({ ...old }) ? old.completed_at : i.updated_at) : null;
        return i;
      },
      get: async (id) => getIssue(id),
      // The prefix is display-only: the number alone identifies the issue.
      getByKey: async (key) => {
        // Same forms as the backend: a case-insensitive prefix, digits with no sign and no leading zero.
        const m = /^([A-Z]{2,5})-([1-9]\d*)$/.exec(key.trim().toUpperCase());
        return s.issues.find((i) => i.id === Number(m?.[2])) ?? fail('NotFound', `issue ${key}`);
      },
      archive: async (id) => {
        const i = getIssue(id);
        if (i.archived_at === null) {
          i.archived_at = now();
          rec(id, 'archived_at', null, i.archived_at);
        }
        return null;
      },
      restore: async (id) => {
        const i = getIssue(id);
        if (i.archived_at !== null) {
          rec(id, 'archived_at', i.archived_at, null);
          i.archived_at = null;
        }
        return null;
      },
      children: async (id) => s.issues.filter((i) => i.parent_id === id),
      // Port of bulk::duplicate: every id must exist before any copy is made.
      duplicate: async (ids) => {
        const from = ids.map(getIssue);
        return from.map((i) => create({ ...structuredClone(i), created_at: undefined, completed_at: undefined }));
      },
      // Port of bulk.rs: per issue the patch (labels merged from add/remove), then archive, then the relation; any failure undoes all.
      bulkUpdate: async (ids, b) => {
        const [before, linked, mark] = [structuredClone(s.issues), [...relations], changes.length];
        try {
          for (const id of ids) {
            const patch = { ...b.patch };
            const [add, drop] = [b.add_labels ?? [], b.remove_labels ?? []];
            if (patch.label_ids == null && (add.length || drop.length)) {
              const keep = getIssue(id).label_ids.filter((l) => !drop.includes(l));
              patch.label_ids = [...keep, ...add.filter((l) => !keep.includes(l))];
            }
            await api.issues.update(id, patch);
            if (b.archive != null) await (b.archive ? api.issues.archive(id) : api.issues.restore(id));
            if (b.add_relation) await api.relations.add({ from: id, to: b.add_relation[1], kind: b.add_relation[0] });
          }
        } catch (e) {
          s.issues.splice(0, s.issues.length, ...before);
          relations.splice(0, relations.length, ...linked);
          changes.length = mark;
          throw e;
        }
        return null;
      },
      parseQuickadd: async (input) => parse(input),
      createFromQuickadd: async (input) => {
        const m = /^\s*\/([^\s/]+)(?=\s|$)\s*([\s\S]*)$/.exec(input);
        // Unknown project or label: its token goes back into the title, nothing is created.
        const p = parse(m ? m[2] : input);
        const project = p.project_slug === null ? undefined : s.projects.find((x) => x.slug === p.project_slug);
        const ids: number[] = [];
        let title = p.title;
        if (p.project_slug !== null && !project) title += ` +${p.project_slug}`;
        for (const name of p.label_names) {
          const l = labels.find((x) => x.name.toLowerCase() === name.toLowerCase());
          if (l) ids.push(l.id);
          else title += ` #${name}`;
        }
        const overrides: NewIssue = {
          title, priority: p.priority, project_id: project?.id, label_ids: ids.length ? ids : undefined,
          start_date: p.start_date, due: p.due, estimate_minutes: p.estimate_minutes, points: p.points,
        };
        if (!m) return create(overrides);
        const t = templates.find((x) => x.slug === m[1]) ?? fail('Invalid', `no template /${m[1]} (known: ${templates.map((x) => `/${x.slug}`).join(', ') || 'none'})`);
        return applyTemplate(t, overrides).issue;
      },
    },
    statuses: {
      list: async () => [...s.statuses].sort(byStatusOrder),
      create: async (name, category, color, icon) => {
        checkIcon(icon);
        const key = colorKey(color)!;
        const st = { id: next(s.statuses), name, category, color: key, color_hex: hexOf(key)!, icon: icon ?? null, position: s.statuses.length, ...flowDefaults(category) };
        s.statuses.push(st);
        return st;
      },
      update: async (id, name, color, position, icon) => {
        const st = find(s.statuses, id, 'status');
        checkIcon(icon);
        const key = colorKey(color) ?? st.color;
        if (name && s.statuses.some((x) => x.id !== id && x.name === name)) fail('Conflict', `status ${name} already exists`);
        return Object.assign(st, { name: name ?? st.name, color: key, color_hex: hexOf(key)!, position: position ?? st.position, icon: icon ?? st.icon });
      },
      setFlow: async (id, definition, flowKind, nudgeDays) => {
        const st = find(s.statuses, id, 'status');
        if (nudgeDays !== null && nudgeDays < 0) fail('Invalid', 'nudge days cannot be negative');
        return Object.assign(st, {
          definition: definition === null ? st.definition : definition.trim(),
          flow_kind: flowKind ?? st.flow_kind,
          nudge_days: nudgeDays === null ? st.nudge_days : nudgeDays > 0 ? nudgeDays : null,
        });
      },
      // Hands the listed statuses' own positions to the new order, like the backend.
      reorder: async (ids) => {
        const list = ids.map((id) => find(s.statuses, id, 'status'));
        if (list.some((x) => x.category !== list[0].category)) fail('Invalid', 'reorder lists one category at a time');
        if (!ids.length || new Set(ids).size !== ids.length) fail('Invalid', 'reorder needs distinct status ids');
        const slots = list.map((x) => x.position).sort((a, b) => a - b);
        list.forEach((x, i) => (x.position = slots[i]));
        return null;
      },
      delete: async (id, moveIssuesTo) => {
        const gone = find(s.statuses, id, 'status');
        find(s.statuses, moveIssuesTo, 'status');
        if (s.statuses.filter((x) => x.category === gone.category).length <= 1)
          fail('Invalid', `cannot delete the last ${gone.category.toLowerCase()} status`);
        for (const i of s.issues) {
          if (i.status_id !== id) continue;
          rec(i.id, 'status_id', id, moveIssuesTo, 'System');
          i.status_id = moveIssuesTo;
        }
        return nothing(remove(s.statuses, id, 'status'));
      },
    },
    project: {
      statusesList: async () => [...projectStatuses].sort((a, b) => a.position - b.position),
      statusesCreate: async (name, category, color, icon) => {
        checkIcon(icon);
        if (!name.trim()) fail('Invalid', 'project status name is empty');
        const key = colorKey(color)!;
        if (projectStatuses.some((x) => x.name === name)) fail('Conflict', `project status ${name} already exists`);
        const st = { id: next(projectStatuses), name, category, color: key, color_hex: hexOf(key)!, icon: icon ?? null, position: Math.max(-1, ...projectStatuses.map((x) => x.position)) + 1 };
        projectStatuses.push(st);
        return st;
      },
      statusesUpdate: async (id, name, color, icon) => {
        const st = find(projectStatuses, id, 'project status');
        checkIcon(icon);
        if (name !== null && !name.trim()) fail('Invalid', 'project status name is empty');
        const key = colorKey(color) ?? st.color;
        if (name && projectStatuses.some((x) => x.id !== id && x.name === name)) fail('Conflict', `project status ${name} already exists`);
        return Object.assign(st, { name: name ?? st.name, color: key, color_hex: hexOf(key)!, icon: icon ?? st.icon });
      },
      statusesReorder: async (ids) => {
        reposition(projectStatuses, ids, (x) => x.id, 'project status');
        return null;
      },
      statusesDelete: async (id, moveProjectsTo) => {
        if (id === moveProjectsTo) fail('Invalid', 'cannot move projects into the deleted status');
        find(projectStatuses, id, 'project status');
        find(projectStatuses, moveProjectsTo, 'project status');
        s.projects.forEach((p) => p.status_id === id && (p.status_id = moveProjectsTo));
        return nothing(remove(projectStatuses, id, 'project status'));
      },
    },
    properties: {
      listDefs: async () => [...s.propertyDefs].sort((a, b) => a.position - b.position),
      createDef: async (d) => {
        if (s.propertyDefs.some((x) => x.key === d.key)) fail('Conflict', `property ${d.key} exists`);
        if (d.applies_to === 'Projects' && d.project_id !== null) fail('Invalid', 'a projects-only property cannot be limited to one project');
        // The position is the next free one whatever the caller sends, like the backend.
        const made = { ...d, applies_to: d.applies_to ?? 'Issues', position: Math.max(-1, ...s.propertyDefs.map((x) => x.position)) + 1 };
        s.propertyDefs.push(made);
        return made;
      },
      renameDef: async (key, newName) => Object.assign(def(key), { name: newName }),
      changeKind: async (key, kind, options) => {
        const d = def(key);
        const bad = s.issues.filter((i) => {
          const v = i.properties[key];
          if (v == null) return false;
          return kind === 'Number' ? Number.isNaN(Number(v)) : kind === 'Select' && !options.some((o) => o.value === v);
        });
        if (bad.length) fail('Conflict', `values do not fit ${kind}: ${bad.map((i) => i.key).join(', ')}`);
        const text = (v: Value): string => (typeof v === 'string' ? v : Array.isArray(v) ? v.map(text).join(', ') : JSON.stringify(v));
        const badProjects = s.projects.filter((p) => {
          const v = p.properties[key];
          if (v == null) return false;
          return kind === 'Number' ? Number.isNaN(Number(v)) : kind === 'Select' && !options.some((o) => o.value === v);
        });
        if (badProjects.length) fail('Conflict', `cannot convert ${key} to ${kind} on projects ${badProjects.map((p) => p.slug).join(', ')}`);
        for (const p of s.projects) {
          const v = p.properties[key];
          if (v == null) continue;
          if (kind === 'Number') p.properties[key] = Number(v);
          else if (kind === 'Text') p.properties[key] = text(v);
        }
        for (const i of s.issues) {
          const v = i.properties[key];
          if (v == null) continue;
          if (kind === 'Number') i.properties[key] = Number(v);
          else if (kind === 'Text') i.properties[key] = text(v);
        }
        return Object.assign(d, { kind, options });
      },
      reorder: async (keys) => {
        reposition(s.propertyDefs, keys, (d) => d.key, 'property');
        return null;
      },
      setAppliesTo: async (key, appliesTo) => {
        const d = def(key);
        const held = (rows: { properties: Record<string, Value> }[]) => rows.filter((r) => r.properties[key] != null).length;
        if (appliesTo === 'Projects' && held(s.issues)) fail('Conflict', `${key} has values on ${held(s.issues)} issues`);
        if (appliesTo === 'Issues' && held(s.projects)) fail('Conflict', `${key} has values on ${held(s.projects)} projects`);
        return Object.assign(d, { applies_to: appliesTo, project_id: appliesTo === 'Projects' ? null : d.project_id });
      },
      setScope: async (key, project) => {
        const d = def(key);
        if (d.applies_to === 'Projects' && project !== null) fail('Invalid', 'a projects-only property cannot be limited to one project');
        const bad = s.issues.filter((i) => i.properties[key] != null && project !== null && i.project_id !== project);
        if (bad.length) fail('Conflict', `${key} has values on issues outside the project: ${bad.map((i) => i.key).join(', ')}`);
        return Object.assign(d, { project_id: project });
      },
      deleteDef: async (key) => {
        s.propertyDefs.splice(s.propertyDefs.indexOf(def(key)), 1);
        s.issues.forEach((i) => delete i.properties[key]);
        s.projects.forEach((p) => delete p.properties[key]);
        [...views, ...viewTemplates].forEach((v) => (v.def = scrubDef(v.def, (k) => k === key)));
        return null;
      },
      setValue: async (issue, key, value) => {
        const d = def(key);
        const i = getIssue(issue);
        if (d.applies_to === 'Projects') fail('Invalid', `${key} applies to projects only`);
        if (d.project_id !== null && d.project_id !== i.project_id) fail('Invalid', `${key} is scoped to another project than ${i.key}`);
        if (value !== null && !validValue(d, value)) fail('Invalid', `${JSON.stringify(value)} is not a valid ${d.kind} for ${key}`);
        const old = i.properties[key] ?? null;
        if (JSON.stringify(old) !== JSON.stringify(value)) {
          if (value === null) delete i.properties[key];
          else i.properties[key] = value;
          i.updated_at = now();
          rec(issue, `properties.${key}`, old, value);
        }
        return i;
      },
    },
    projects: {
      list: async () => s.projects,
      create: async (slug, name, icon, color) => {
        if (s.projects.some((p) => p.slug === slug)) fail('Db', 'error returned from database: (code: 2067) UNIQUE constraint failed: projects.slug');
        checkIcon(icon);
        const p = { id: next(s.projects), slug, name, description: null, doc_link: null, start_date: null, target_date: null, archived_at: null, created_at: now(), updated_at: now(), icon: icon ?? null, color: colorKey(color), ...projectDefaults() };
        s.projects.push(p);
        return p;
      },
      update: async (p) => {
        checkIcon(p.icon);
        find(s.projects, p.id, 'project');
        if (s.projects.some((x) => x.id !== p.id && x.slug === p.slug)) fail('Db', 'error returned from database: (code: 2067) UNIQUE constraint failed: projects.slug');
        if (p.priority > 4) fail('Invalid', 'priority is 0 to 4');
        find(projectStatuses, p.status_id, 'project status');
        for (const [k, v] of Object.entries(p.properties)) {
          const d = def(k);
          if (d.applies_to === 'Issues') fail('Invalid', `${k} does not apply to projects`);
          if (!validValue(d, v)) fail('Invalid', `${JSON.stringify(v)} is not a valid ${d.kind} for ${k}`);
        }
        for (const l of p.label_ids) find(labels, l, 'label');
        return upsert(s.projects, { ...p, created_at: find(s.projects, p.id, 'project').created_at, updated_at: now(), color: colorKey(p.color), label_ids: [...new Set(p.label_ids)].sort((a, b) => a - b) }, 'project');
      },
      delete: async (id) => {
        remove(s.projects, id, 'project');
        s.issues.forEach((i) => i.project_id === id && (i.project_id = null));
        return null;
      },
      progress: async (id) => progressOf(id),
      listMilestones: async (project) => milestones.filter((m) => m.project_id === project),
      createMilestone: async (project, name, targetDate) => {
        const m = { id: next(milestones), project_id: find(s.projects, project, 'project').id, name, target_date: targetDate };
        milestones.push(m);
        return m;
      },
      updateMilestone: async (m) => upsert(milestones, m, 'milestone'),
      deleteMilestone: async (id) => {
        remove(milestones, id, 'milestone');
        s.issues.forEach((i) => i.milestone_id === id && (i.milestone_id = null));
        return null;
      },
    },
    cycles: {
      list: async () => cycles,
      create: async (name, startDate, endDate, icon, color) => {
        if (endDate < startDate) fail('Invalid', `cycle ends ${endDate} before it starts ${startDate}`);
        checkIcon(icon);
        const c = { id: next(cycles), number: Math.max(0, ...cycles.map((x) => x.number)) + 1, name, start_date: startDate, end_date: endDate, icon: icon ?? null, color: colorKey(color) };
        cycles.push(c);
        return c;
      },
      update: async (c) => upsert(cycles, c, 'cycle'),
      delete: async (id) => {
        remove(cycles, id, 'cycle');
        s.issues.forEach((i) => i.cycle_id === id && (i.cycle_id = null));
        return null;
      },
    },
    events: {
      list: async (from, to) =>
        events
          .filter((e) => !e.archived && (from === null || (e.ends_at ?? e.starts_at).slice(0, 10) >= from) && (to === null || e.starts_at.slice(0, 10) <= to))
          .sort((a, b) => a.starts_at.localeCompare(b.starts_at) || a.id - b.id),
      get: async (id) => ({ ...find(events, id, 'event') }),
      create: async (input) => {
        const e = eventOf(next(events), input, []);
        events.push(e);
        return { ...e };
      },
      update: async (id, input) => {
        const old = eventOf(id, input, []);
        const e = find(events, id, 'event');
        return { ...Object.assign(e, { ...old, issue_ids: e.issue_ids }) };
      },
      delete: async (id) => nothing(remove(events, id, 'event')),
      link: async (event, issue) => {
        const e = find(events, event, 'event');
        getIssue(issue);
        if (!e.issue_ids.includes(issue)) e.issue_ids = [...e.issue_ids, issue].sort((a, b) => a - b);
        return null;
      },
      unlink: async (event, issue) => {
        const e = find(events, event, 'event');
        e.issue_ids = e.issue_ids.filter((x) => x !== issue);
        return null;
      },
    },
    decisions: decisionsApi(decisions, (id) => void getIssue(id), fail),
    templates: {
      list: async () => [...templates],
      create: async (input) => {
        const t = templateOf(next(templates), input);
        templates.push(t);
        return { ...t };
      },
      update: async (id, input) => {
        Object.assign(find(templates, id, 'template'), templateOf(id, input));
        return { ...find(templates, id, 'template') };
      },
      delete: async (id) => nothing(remove(templates, id, 'template')),
      apply: async (id, overrides) => applyTemplate(find(templates, id, 'template'), overrides),
    },
    labels: {
      list: async () => labels,
      create: async (name, color, icon) => {
        if (labels.some((l) => l.name === name)) fail('Db', 'error returned from database: (code: 2067) UNIQUE constraint failed: labels.name');
        checkIcon(icon);
        const key = colorKey(color)!;
        const l = { id: next(labels), name, color: key, color_hex: hexOf(key)!, icon: icon ?? null };
        labels.push(l);
        return l;
      },
      update: async (l) => {
        checkIcon(l.icon);
        const key = colorKey(l.color)!;
        if (labels.some((x) => x.id !== l.id && x.name === l.name)) fail('Db', 'error returned from database: (code: 2067) UNIQUE constraint failed: labels.name');
        return upsert(labels, { ...l, color: key, color_hex: hexOf(key)! }, 'label');
      },
      delete: async (id) => {
        find(labels, id, 'label');
        for (const i of s.issues) if (i.label_ids.includes(id)) rec(i.id, 'label', id, null);
        remove(labels, id, 'label');
        s.issues.forEach((i) => (i.label_ids = i.label_ids.filter((x) => x !== id)));
        s.projects.forEach((p) => (p.label_ids = p.label_ids.filter((x) => x !== id)));
        return null;
      },
      addLabel: async (issue, label) => {
        find(labels, label, 'label');
        const i = getIssue(issue);
        if (!i.label_ids.includes(label)) {
          i.label_ids = [...i.label_ids, label].sort((a, b) => a - b);
          rec(issue, 'label', null, label);
        }
        return null;
      },
      removeLabel: async (issue, label) => {
        const i = getIssue(issue);
        if (i.label_ids.includes(label)) rec(issue, 'label', label, null);
        i.label_ids = i.label_ids.filter((x) => x !== label);
        return null;
      },
    },
    attachments: attachmentsApi(fail),
    comments: {
      list: async (issue) => comments.filter((c) => c.issue_id === issue),
      addComment: async (issue, body) => {
        const at = now();
        const c = { id: next(comments), issue_id: getIssue(issue).id, body, created_at: at, updated_at: at };
        comments.push(c);
        return c;
      },
      update: async (id, body) => Object.assign(find(comments, id, 'comment'), { body, updated_at: now() }),
      delete: async (id) => nothing(remove(comments, id, 'comment')),
    },
    relations: {
      add: async (given) => {
        if (given.from === given.to) fail('Invalid', 'an issue cannot relate to itself');
        const rel = { ...given, kind: kinds.resolve(given.kind) };
        if (!s.issues.some((i) => i.id === rel.from) || !s.issues.some((i) => i.id === rel.to)) fail('NotFound', `issue ${rel.from} or ${rel.to}`);
        const same = (r: Relation) => r.from === rel.from && r.to === rel.to && r.kind === rel.kind;
        if (relations.some(same)) return null;
        if (rel.kind === 'blocks') {
          // Adding from -> to must not let `to` reach `from` through blocking edges.
          const seen = new Set<number>();
          const reach = (id: number): boolean => id === rel.from || (!seen.has(id) && seen.add(id) && relations.some((r) => r.kind === 'blocks' && r.from === id && reach(r.to)));
          if (reach(rel.to)) fail('Conflict', 'would create a dependency cycle');
        }
        relations.push(rel);
        rec(rel.from, 'relation', null, rel);
        return null;
      },
      remove: async (given) => {
        const rel = { ...given, kind: kinds.resolve(given.kind) };
        const at = relations.findIndex((r) => r.from === rel.from && r.to === rel.to && r.kind === rel.kind);
        if (at >= 0) rec(rel.from, 'relation', relations.splice(at, 1)[0], null);
        return null;
      },
      forIssue: async (id) => relations.filter((r) => r.from === id || r.to === id),
    },
    relation: {
      kindsList: kinds.list, kindsCreate: kinds.create, kindsUpdate: kinds.update, kindsDelete: kinds.delete,
    },
    schedule: {
      bars: async (d) => query(d).flatMap((i) => barFor(i) ?? []),
      criticalPath: async (d) => criticalPath(query(d)),
      shiftDependents: async (moved) => shiftDependents(moved),
    },
    views: {
      // Fresh copies through JSON, as the backend sends them: a view that reloads in place must see new objects to redraw.
      query: async (d) => JSON.parse(JSON.stringify(query(d))),
      queryProjects: async (d) => JSON.parse(JSON.stringify(queryProjects(d))),
      list: async () => views,
      stats: async (d) =>
        query(d).map((i) => {
          const kids = s.issues.filter((c) => c.parent_id === i.id && !c.archived_at);
          return {
            issue_id: i.id, sub_total: kids.length, sub_done: kids.filter((c) => category(c) === 'Completed').length,
            relations: relations.filter((r) => r.from === i.id || r.to === i.id).length,
            comments: comments.filter((c) => c.issue_id === i.id).length,
          };
        }),
      save: (id, name, d, icon, color) => saveView(id, name, d, icon, color, 'issue'),
      saveProject: (id, name, d, icon, color) => saveView(id, name, d, icon, color, 'project'),
      // Not in `mutating` below: picking a tab makes no undo step, as in the backend.
      setLayout: async (id, layout) => {
        const v = find(views, id, 'view');
        v.def = { ...v.def, layout };
        return null;
      },
      createFromTemplate: async (templateId, name) => {
        const t = find(viewTemplates, templateId, 'view template');
        const v = { id: next(views), name: name ?? t.name, entity: 'issue' as const, def: withDisplay(t.def), position: Math.max(-1, ...views.map((x) => x.position)) + 1, icon: t.icon, color: t.color, template_id: t.id, folder_id: null };
        views.push(v);
        return v;
      },
      saveAsTemplate: async (viewId, name) => {
        const v = find(views, viewId, 'view');
        const t = { id: next(viewTemplates), name: name ?? v.name, icon: v.icon, color: v.color, def: withDisplay(v.def), position: Math.max(-1, ...viewTemplates.map((x) => x.position)) + 1 };
        viewTemplates.push(t);
        return t;
      },
      resetToTemplate: async (viewId) => {
        const v = find(views, viewId, 'view');
        if (v.template_id === null) fail('Invalid', 'this view was not made from a template');
        v.def = withDisplay(find(viewTemplates, v.template_id!, 'view template').def);
        return v;
      },
      delete: async (id) => nothing(remove(views, id, 'view')),
      reorder: async (ids) => {
        reposition(views, ids, (v) => v.id, 'view');
        views.sort((a, b) => a.position - b.position);
        return null;
      },
    },
    folders: foldersApi(
      folders, placed, views,
      (sec) => ({ pin: pins.map((x) => x.id), view: views.map((x) => x.id), project: s.projects.map((x) => x.id), cycle: cycles.map((x) => x.id) })[sec],
      (sec, id) => (sec === 'pin' ? pins.find((x) => x.id === id)!.position : 1_000_000),
      (sec, id, position) => { if (sec === 'pin') pins.find((x) => x.id === id)!.position = position; },
      fail,
    ),
    view: {
      templatesList: async () => viewTemplates,
      templatesCreate: async (name, d, icon, color) => {
        checkIcon(icon);
        const t = { id: next(viewTemplates), name, icon: icon ?? null, color: colorKey(color), def: withDisplay(d), position: Math.max(-1, ...viewTemplates.map((x) => x.position)) + 1 };
        viewTemplates.push(t);
        return t;
      },
      templatesUpdate: async (t) => {
        checkIcon(t.icon);
        const row = find(viewTemplates, t.id, 'view template');
        return Object.assign(row, { name: t.name, icon: t.icon, color: colorKey(t.color), def: withDisplay(t.def) });
      },
      templatesDelete: async (id) => {
        remove(viewTemplates, id, 'view template');
        for (const v of views) if (v.template_id === id) v.template_id = null;
        if (settings.get('default_view_template') === id) settings.delete('default_view_template');
        return null;
      },
    },
    insights: {
      progress: async (scope, measure, from, to) => {
        const { tls, range } = scoped(scope, measure);
        const [f, t] = [from ?? range[0], to ?? range[1]];
        if (t < f) fail('Invalid', `range ends (${t}) before it starts (${f})`);
        if ((Date.parse(t) - Date.parse(f)) / 86_400_000 >= insights.MAX_DAYS) fail('Invalid', `range ${f} to ${t} must be 1 to ${insights.MAX_DAYS} days`);
        return insights.series(tls, measure, f, t);
      },
      velocity: async (scope, measure, periods) => {
        const { tls } = scoped(scope, measure);
        const now = zoneToday();
        const started = [...cycles].sort((a, b) => a.number - b.number).filter((c) => c.start_date <= now);
        const spans = cycles.length
          ? started.slice(Math.max(0, started.length - periods)).map((c) => ({ start: c.start_date, end: c.end_date, label: c.name ?? `Cycle ${c.number}` }))
          : insights.weekSpans(now, periods);
        return insights.velocity(tls, spans);
      },
    },
    forecast: {
      for: async (q) => forecast(q, query(withDisplay({ filter: q.filter, layout: 'List' })), { changes, statuses: s.statuses, cycles, labels, tz: workspace.timezone, today: zoneToday() }, fail),
    },
    metrics: {
      for: async (q) => metrics(q, query(withDisplay({ filter: q.filter, layout: 'List' })), { changes, statuses: s.statuses, cycles, labels, relations, everyone: query(withDisplay({ filter: null, layout: 'List' })), tz: workspace.timezone, today: zoneToday() }, fail),
    },
    search: {
      // shortcut: results come in id order; the backend ranks by bm25. Upgrade when a screen depends on relevance order.
      search: async (text, limit) => {
        const words = (t: string) => t.toLowerCase().match(/[\p{L}\p{N}]+/gu) ?? [];
        const phrases = text.split(/\s+/).filter(Boolean).map(words).filter((p) => p.length);
        // Like the backend's "word"* phrases: the words are adjacent, and the last one may be only the start of a word.
        const has = (hay: string[], p: string[]) => hay.some((_, i) => p.every((w, k) => (k === p.length - 1 ? hay[i + k]?.startsWith(w) : hay[i + k] === w)));
        return phrases.length
          ? s.issues.filter((i) => {
              const hay = words([i.title, i.description, ...comments.filter((c) => c.issue_id === i.id).map((c) => c.body)].join(' '));
              return !i.archived_at && phrases.every((p) => has(hay, p));
            }).slice(0, limit)
          : [];
      },
    },
    pins: {
      pin: async (kind, target, title) => {
        checkPin(kind, target);
        const have = pins.find((p) => p.kind === kind && p.target === target);
        if (have) return have;
        const p = { id: next(pins), kind, target, title: title?.trim() ? title : null, position: Math.max(-1, ...pins.map((x) => x.position)) + 1 };
        pins.push(p);
        return p;
      },
      set: async (kind, targets, pinned) => {
        targets.forEach((t) => checkPin(kind, t));
        for (const t of targets) {
          const have = pins.find((p) => p.kind === kind && p.target === t);
          if (pinned) have || pins.push({ id: next(pins), kind, target: t, title: null, position: Math.max(-1, ...pins.map((x) => x.position)) + 1 });
          else if (have) remove(pins, have.id, 'pin');
        }
        return null;
      },
      unpin: async (id) => nothing(remove(pins, id, 'pin')),
      reorder: async (ids) => {
        if (ids.length !== pins.length || pins.some((p) => ids.filter((x) => x === p.id).length !== 1)) fail('Invalid', 'reorder must list every current pin exactly once');
        ids.forEach((id, position) => (find(pins, id, 'pin').position = position));
        return null;
      },
      // Like the backend, a pin whose target is gone is dropped.
      listResolved: async () => {
        const prefix = settings.get('issue_prefix');
        const resolved = [...pins].sort((a, b) => a.position - b.position || a.id - b.id).flatMap((pin) => {
          const t = pin.target;
          let hit: [string, boolean, string] | null = null;
          if (pin.kind === 'Issue') {
            const n = Number(/-(\d+)$/.exec(t)?.[1]);
            const i = s.issues.find((x) => x.id === n);
            hit = i ? [i.title, i.archived_at !== null, `/issue/${prefix}-${n}`] : null;
          } else if (pin.kind === 'Project') {
            const p = s.projects.find((x) => x.slug === t);
            hit = p ? [p.name, p.archived_at !== null, `/project/${t}`] : null;
          } else if (pin.kind === 'View') {
            const v = views.find((x) => String(x.id) === t);
            hit = v ? [v.name, false, `/view/${v.id}`] : null;
          } else if (pin.kind === 'Cycle') {
            const c = cycles.find((x) => String(x.number) === t);
            hit = c ? [c.name ?? `Cycle ${c.number}`, false, `/cycles#${c.number}`] : null;
          } else if (pin.kind === 'Folder') {
            const f = folders.find((x) => String(x.id) === t);
            hit = f ? [f.name, false, '/views'] : null;
          } else hit = [t, false, t];
          return hit ? [{ pin, label: pin.title ?? hit[0], archived: hit[1], route: hit[2] }] : [];
        });
        for (const p of [...pins]) if (!resolved.some((r) => r.pin.id === p.id)) remove(pins, p.id, 'pin');
        return resolved;
      },
    },
    blocks: {
      create: async (issue, start, end) => {
        const b = { id: next(blocks), issue_id: getIssue(issue).id, start_utc: start, end_utc: end };
        blocks.push(b);
        return b;
      },
      move: async (id, start, end) => Object.assign(find(blocks, id, 'work block'), { start_utc: start, end_utc: end }),
      delete: async (id) => nothing(remove(blocks, id, 'work block')),
      between: async (from, to) => blocks.filter((b) => b.start_utc < to && b.end_utc > from),
    },
    history: {
      listFor: async (entity, entityId) => changes.filter((c) => c.entity === entity && c.entity_id === entityId),
      undo: async () => undoRedo(true),
      redo: async () => undoRedo(false),
      undoStatus: async () => status(),
    },
    workspace: {
      status: async () => ({ ...workspace }),
      // Same order of checks as core: name, key, zone. The key is a display prefix, so every issue key follows it.
      setup: async (name, key, timezone) => {
        if (!name.trim()) fail('Invalid', 'workspace name must not be empty');
        if (!/^[A-Z]{2,5}$/.test(key)) fail('Invalid', `issue key \`${key}\` must be 2 to 5 letters A-Z`);
        if (timezone != null) {
          try {
            new Intl.DateTimeFormat('en', { timeZone: timezone });
          } catch {
            fail('Invalid', `unknown time zone ${timezone}`);
          }
        }
        if (!workspace.setup_done && !viewTemplates.length) {
          viewTemplates.push(...seedViewTemplates(s.propertyDefs.map((d) => d.key), day(7)));
        }
        workspace = { setup_done: true, name: name.trim(), key, timezone: timezone ?? workspace.timezone };
        settings.set('issue_prefix', key);
        for (const i of s.issues) i.key = `${key}-${i.id}`;
        return { ...workspace };
      },
    },
    settings: {
      get: async (key) => settings.get(key) ?? null,
      set: async (key, value) => {
        if (key === 'issue_fields') checkIssueFields(value);
        if (key === 'default_status' && !(typeof value === 'number' && s.statuses.some((st) => st.id === value)))
          fail('Invalid', `setting default_status must be ${typeof value === 'number' ? 'an existing status' : 'a status id'}`);
        const allowed = choices[key];
        if (allowed && !(typeof value === 'string' && allowed.includes(value))) fail('Invalid', `setting ${key} must be one of ${allowed.join(', ')}`);
        return nothing(settings.set(key, value));
      },
    },
    backup: {
      snapshot: async (dir, reason) => {
        const path = `${dir}/snapshots/mock-${reason}.db`;
        backups.unshift(path);
        return path;
      },
      list: async (dir) => backups.filter((p) => p.startsWith(`${dir}/`)),
    },
    import: {
      csvPlan: async (csv, mapping) =>
        planCsv(csv, mapping, {
          statuses: s.statuses.map((x) => x.name), labels: labels.map((x) => x.name), projects: s.projects.map((x) => x.name),
          propertyKeys: s.propertyDefs.map((d) => d.key), cycleStarts: cycles.map((c) => c.start_date), imported: new Set(importLog.map((x) => x.external_id)),
        }),
      apply: async (plan) => {
        const byName = <T extends { name: string }>(rows: T[], name: string) => rows.find((r) => r.name.toLowerCase() === name.toLowerCase());
        for (const st of plan.statuses ?? []) {
          if (!byName(s.statuses, st.name)) {
            const made: Status = { id: next(s.statuses), name: st.name, category: st.category, position: s.statuses.length, color: 'gray', color_hex: hexOf('gray')!, icon: null, ...flowDefaults(st.category) };
            s.statuses.push(made);
            keptStatuses.push(made);
          }
        }
        for (const l of plan.labels ?? []) if (!byName(labels, l)) labels.push({ id: next(labels), name: l, color: 'gray', color_hex: hexOf('gray')!, icon: null });
        for (const name of plan.projects ?? []) {
          if (byName(s.projects, name)) continue;
          const base = projectSlug(name);
          let slug = base;
          for (let n = 2; s.projects.some((p) => p.slug === slug); n++) slug = `${base}-${n}`;
          s.projects.push({ id: next(s.projects), slug, name, description: null, doc_link: null, start_date: null, target_date: null, archived_at: null, created_at: now(), updated_at: now(), icon: null, color: null, ...projectDefaults() });
        }
        for (const c of plan.cycles ?? []) {
          if (!cycles.some((x) => x.start_date === c.start_date)) cycles.push({ id: next(cycles), number: Math.max(0, ...cycles.map((x) => x.number)) + 1, name: c.name, start_date: c.start_date, end_date: c.end_date, icon: null, color: null });
        }
        for (const d of plan.property_defs ?? []) if (!s.propertyDefs.some((x) => x.key === d.key)) s.propertyDefs.push({ ...d, position: Math.max(-1, ...s.propertyDefs.map((x) => x.position)) + 1 });
        const [ids, mark] = [new Map<string, number>(), changes.length];
        const warnings: string[] = [];
        const known = (ext: string) => ids.get(ext) ?? importLog.find((x) => x.external_id === ext)?.issue_id;
        for (const i of plan.issues ?? []) {
          if (known(i.external_id) !== undefined) {
            warnings.push(`${i.external_id} was already imported`);
            continue;
          }
          const made = create({
            title: i.title, description: i.description, priority: i.priority, start_date: i.start_date, due: i.due_date ? { Date: i.due_date } : null,
            estimate_minutes: i.estimate_minutes, points: i.points, properties: i.properties,
            status_id: i.status ? byName(s.statuses, i.status)?.id : undefined, project_id: i.project ? byName(s.projects, i.project)?.id : undefined,
            cycle_id: i.cycle ? cycles.find((c) => c.start_date === i.cycle)?.id : undefined,
            label_ids: i.labels.flatMap((l) => byName(labels, l)?.id ?? []),
          });
          importLog.push({ external_id: i.external_id, issue_id: made.id });
          ids.set(i.external_id, made.id);
        }
        for (const i of (plan.issues ?? []).filter((x) => ids.has(x.external_id) && x.parent)) {
          const parent = known(i.parent!);
          if (parent === undefined) warnings.push(`${i.external_id}: parent ${i.parent} was not imported`);
          else getIssue(ids.get(i.external_id)!).parent_id = parent;
        }
        for (const r of plan.relations ?? []) {
          const [from, to] = [known(r.from), known(r.to)];
          if (from === undefined || to === undefined) warnings.push(`relation ${r.from} -> ${r.to} skipped: an issue was not imported`);
          else if (!relations.some((x) => x.from === from && x.to === to && x.kind === kinds.resolve(r.kind))) relations.push({ from, to, kind: kinds.resolve(r.kind) });
        }
        // Like the real import, every row it wrote says so (metrics split by source).
        for (const c of changes.slice(mark)) c.source = 'Migration';
        return { created: ids.size, warnings };
      },
    },
    export: { now: ok, pull: async () => ({ created: 0, warnings: [] }) },
    // Same checks as the real command; a source containing "bad" stands for an export that does not parse.
    restore: {
      fromExport: async (source) => {
        const text = source.trim();
        if (!text) fail('Invalid', 'enter the export folder or the Git address');
        if (/^[a-z]+:\/\//i.test(text) && !/^(https|ssh):\/\//.test(text)) fail('Invalid', 'use an https:// or ssh:// Git URL, or git@host:path');
        await new Promise((r) => setTimeout(r, 400));
        if (text.includes('bad')) fail('Invalid', 'statuses.yaml: invalid export file');
        workspace = { ...workspace, setup_done: true };
        return text;
      },
    },
    sync: { status: async () => off, now: async () => off },
    gcal: { connect: ok, listCalendars: async () => [['primary', 'Primary']] },
    reminders: { listLists: async () => [['1', 'Reminders']] },
  };
  const commandOf = (mod: string, fn: string) => `${mod}_${fn.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)}`;
  const mutating = (
    'issues.create issues.update issues.bulkUpdate issues.duplicate issues.archive issues.restore issues.createFromQuickadd statuses.create statuses.update statuses.reorder statuses.setFlow statuses.delete project.statusesCreate project.statusesUpdate project.statusesReorder project.statusesDelete properties.setAppliesTo ' +
    'properties.createDef properties.renameDef properties.changeKind properties.reorder properties.setScope properties.deleteDef properties.setValue ' +
    'projects.create projects.update projects.delete projects.createMilestone projects.updateMilestone projects.deleteMilestone ' +
    'cycles.create cycles.update cycles.delete events.create events.update events.delete events.link events.unlink decisions.create decisions.answer decisions.withdraw decisions.link decisions.unlink templates.create templates.update templates.delete templates.apply ' +
    'labels.create labels.update labels.delete labels.addLabel labels.removeLabel comments.addComment comments.update comments.delete ' +
    'relations.add relations.remove relation.kindsCreate relation.kindsUpdate relation.kindsDelete views.save views.saveProject views.delete views.reorder folders.create folders.rename folders.delete folders.place folders.placeItem views.createFromTemplate views.saveAsTemplate views.resetToTemplate ' +
    'view.templatesCreate view.templatesUpdate view.templatesDelete pins.pin pins.unpin pins.reorder pins.set import.apply'
  ).split(' ');
  for (const name of mutating) {
    const [mod, fn] = name.split('.');
    const group = (api as unknown as Record<string, Record<string, (...a: unknown[]) => Promise<unknown>>>)[mod];
    const run = group[fn];
    group[fn] = (...args) =>
      tracked((r) => stepLabel(commandOf(mod, fn), args, r), () => run(...args));
  }
  return api;
}

const statuses: Status[] = (
  [
    ['Backlog', 'Backlog', 'gray'], ['Todo', 'Unstarted', 'purple'], ['In Progress', 'Started', 'yellow'],
    ['Done', 'Completed', 'indigo'], ['Canceled', 'Canceled', 'gray'],
  ] as const
).map(([name, category, color], position) => ({ id: position + 1, name, category, position, color, color_hex: hexOf(color)!, icon: null, ...flowDefaults(category) }));

export const projectStatusSeed: ProjectStatus[] = (
  [
    ['Planned', 'Planned', 'gray'], ['In progress', 'Started', 'amber'], ['Paused', 'Paused', 'orange'],
    ['Done', 'Completed', 'indigo'], ['Cancelled', 'Cancelled', 'gray'],
  ] as [string, ProjectStatusCategory, string][]
).map(([name, category, color], position) => ({ id: position + 1, name, category, position, color, color_hex: hexOf(color)!, icon: null }));

const project = (id: number, slug: string, name: string): Project =>
  ({ id, slug, name, description: null, doc_link: null, start_date: null, target_date: null, archived_at: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(), icon: null, color: null, status_id: 1, priority: 0, label_ids: [], properties: {} });
const day = (n: number) => addDays(new Date().toISOString().slice(0, 10), n);
// The last two are under way with dates; the first has a start only (the devserver seed does the same).
const projects: Project[] = [
  { ...project(1, 'website', 'Website relaunch'), start_date: day(-5) },
  { ...project(2, 'mobile', 'Mobile app'), status_id: 2, priority: 3, start_date: day(-3), target_date: day(45) },
  { ...project(3, 'office', 'Office move'), status_id: 2, priority: 2, start_date: day(-14), target_date: day(30) },
];

const labels: Label[] = (
  ['Bug:red:flag', 'Feature:blue:bolt', 'Design:purple:star', 'Research:teal:target', 'Docs:green:bookmark', 'Ops:orange:tag'] as const
).map((x, i) => {
  const [name, color, icon] = x.split(':');
  return { id: i + 1, name, color, color_hex: hexOf(color)!, icon: `icon:${icon}` };
});

// Everything dated is relative to today (UTC), so the workspace always has overdue and this-week work.
// The devserver seed (crates/devserver/src/seed.rs) builds the same data; the contract tests compare the two.
const cycles: Cycle[] = [
  { id: 1, number: 1, name: 'Sprint 1', start_date: day(-6), end_date: day(7), icon: null, color: null },
  { id: 2, number: 2, name: 'Sprint 2', start_date: day(8), end_date: day(21), icon: null, color: null },
];
const milestones: Milestone[] = [
  { id: 1, project_id: 1, name: 'Soft launch', target_date: day(21) },
  { id: 2, project_id: 2, name: 'Store release', target_date: day(30) },
];

const propertyDefs: PropertyDef[] = [
  { key: 'area', name: 'Area', kind: 'Select', options: [{ value: 'home', color: '#4caf50' }, { value: 'work', color: '#2196f3' }], project_id: null, position: 0, applies_to: 'Issues' },
  { key: 'effort', name: 'Effort', kind: 'Number', options: [], project_id: null, position: 1, applies_to: 'Issues' },
  { key: 'review_on', name: 'Review on', kind: 'Date', options: [], project_id: null, position: 2, applies_to: 'Issues' },
];

type Row = {
  title: string; status: number; priority: number; project: number | null; start?: number; due?: number; hours?: number;
  labels?: number[]; parent?: number; cycle?: number; milestone?: number; description?: string; area?: string; effort?: number;
};
const rows: Row[] = [
  { title: 'Redesign homepage hero', status: 3, priority: 3, project: 1, start: -5, due: 2, hours: 16, labels: [3], cycle: 1, milestone: 1, area: 'work', effort: 13, description: 'Replace the stock photo with the new brand illustration and tighten the headline.' },
  { title: 'Write launch announcement', status: 2, priority: 2, project: 1, start: 0, due: 5, hours: 2, labels: [5], milestone: 1 },
  { title: 'Fix broken links in footer', status: 4, priority: 1, project: 1, due: -3, hours: 1, labels: [1] },
  { title: 'Migrate blog to new CMS', status: 3, priority: 4, project: 1, start: -10, due: -2, hours: 40, labels: [6], cycle: 1, area: 'work', effort: 21, description: 'Export posts, redirect the old URLs and check every embedded image.' },
  { title: 'Set up analytics dashboard', status: 1, priority: 0, project: 1, hours: 4, labels: [2] },
  { title: 'Audit page speed', status: 2, priority: 2, project: 1, start: 1, due: 4, hours: 3, labels: [4], effort: 8 },
  { title: 'Mobile onboarding flow', status: 3, priority: 3, project: 2, start: -3, due: 6, hours: 24, labels: [2, 3], cycle: 1, milestone: 2, area: 'work', effort: 34, description: 'Three screens: welcome, permissions and account setup. Skippable at every step.' },
  { title: 'Welcome screen illustrations', status: 3, priority: 2, project: 2, start: -3, due: 1, hours: 8, labels: [3], parent: 7, cycle: 1 },
  { title: 'Permission prompts copy', status: 2, priority: 1, project: 2, start: 2, due: 5, hours: 1, labels: [5], parent: 7 },
  { title: 'Crash on launch for Android 9', status: 3, priority: 4, project: 2, due: -1, hours: 4, labels: [1], cycle: 1 },
  { title: 'Push notification settings', status: 2, priority: 2, project: 2, start: 3, due: 9, hours: 16, labels: [2], cycle: 2 },
  { title: 'Offline mode research', status: 1, priority: 1, project: 2, hours: 8, labels: [4], description: 'Compare local-first sync options and write up the trade-offs.' },
  { title: 'Dark mode icons', status: 4, priority: 2, project: 2, start: -8, due: -4, hours: 4, labels: [3] },
  { title: 'Update app store listing', status: 2, priority: 2, project: 2, due: 3, hours: 1, labels: [5], parent: 7, cycle: 2 },
  { title: 'Confirm new office lease', status: 4, priority: 3, project: 3, start: -14, due: -9 },
  { title: 'Order standing desks', status: 3, priority: 2, project: 3, start: -2, due: 4, hours: 2, labels: [6], cycle: 1, area: 'home' },
  { title: 'Book movers', status: 2, priority: 3, project: 3, start: 2, due: 11, hours: 1, area: 'home' },
  { title: 'Plan seating chart', status: 2, priority: 2, project: 3, start: 4, due: 10, hours: 4, labels: [3] },
  { title: 'Set up network and Wi-Fi', status: 1, priority: 3, project: 3, due: 14, hours: 16, labels: [6], description: 'Two access points per floor, a guest network and a printer VLAN.' },
  { title: 'Order access points', status: 1, priority: 2, project: 3, hours: 8, labels: [6], parent: 19 },
  { title: 'Send address change notices', status: 2, priority: 2, project: 3, due: 7, hours: 1, labels: [5] },
  { title: 'Cancel old storage unit', status: 5, priority: 0, project: 3 },
  { title: 'Rename Contact page', status: 2, priority: 1, project: null, hours: 0.5 },
  { title: 'Team retro notes', status: 1, priority: 0, project: null, labels: [4] },
];
// Story points by issue number; the issues not listed have none (crates/devserver/src/seed.rs lists the same).
const POINTS: Record<number, number> = { 1: 5, 3: 3, 4: 8, 5: 2, 7: 13, 8: 3, 10: 5, 11: 8, 13: 5, 14: 1, 15: 2, 16: 5, 17: 3, 19: 8, 20: 3, 23: 1 };
// Status moves of the last three weeks, [issue, days ago, from, to]; every issue starts in Todo (2) and the last
// move ends in its status now. Point revisions are [issue, days ago, from, to] too. Same rows as the real seed.
const MOVES: [number, number, number, number][] = [
  [3, 22, 2, 3], [3, 17, 3, 4], [4, 20, 2, 3], [10, 18, 2, 3], [10, 11, 3, 4], [10, 5, 4, 3], [13, 15, 2, 3], [13, 9, 3, 4],
  [15, 14, 2, 3], [15, 12, 3, 4], [1, 12, 2, 3], [7, 9, 2, 3], [8, 8, 2, 3], [16, 5, 2, 3], [22, 4, 2, 5],
];
const REVISIONS: [number, number, number, number][] = [[4, 12, 5, 8], [7, 7, 8, 13]];
const history: Change[] = [
  ...MOVES.map(([n, ago, from, to]) => [n, ago, 'status_id', from, to] as const),
  ...REVISIONS.map(([n, ago, from, to]) => [n, ago, 'points', from, to] as const),
].map(([n, ago, field, old_value, new_value], id) => ({
  id: id + 1, at: `${day(-ago)}T12:00:00Z`, source: 'Ui', entity: 'issue', entity_id: String(n), field, old_value, new_value,
}));
const issues: Issue[] = rows.map((r, k) => {
  const n = k + 1;
  const created = `${day(-30 + n)}T09:00:00Z`;
  const finished = statuses[r.status - 1].category === 'Completed' || statuses[r.status - 1].category === 'Canceled';
  return {
    id: n, key: `ABC-${n}`, title: r.title, description: r.description ?? '', status_id: r.status, priority: r.priority,
    project_id: r.project, milestone_id: r.milestone ?? null, parent_id: r.parent ?? null, cycle_id: r.cycle ?? null,
    label_ids: r.labels ?? [], start_date: r.start === undefined ? null : day(r.start),
    due: r.due === undefined ? null : { Date: day(r.due) }, estimate_minutes: r.hours === undefined ? null : r.hours * 60,
    points: POINTS[n] ?? null,
    properties: { ...(r.area && { area: r.area }), ...(r.effort && { effort: r.effort }) } as Issue['properties'],
    created_at: created, updated_at: created, completed_at: finished ? created : null, archived_at: null,
  };
});
const relations: Relation[] = [
  { from: 4, to: 6, kind: 'blocks' }, { from: 16, to: 18, kind: 'blocks' }, { from: 10, to: 11, kind: 'related' },
];
const templates: IssueTemplate[] = [
  {
    id: 1, name: 'Bug', slug: 'bug', icon: 'icon:flag', color: 'red',
    body: {
      title_prefix: 'Bug: ', sub_issues: [{ title: 'Reproduce', fields: fieldsOf({ priority: 2 }) }],
      fields: fieldsOf({ description: 'Steps to reproduce:\n\nExpected:\n\nActual:', priority: 2, label_ids: [1] }),
    },
  },
  {
    id: 2, name: 'Research idea', slug: 'research', icon: 'icon:target', color: 'teal',
    body: { title_prefix: 'Research: ', sub_issues: [], fields: fieldsOf({ description: 'Question:\n\nWhy it matters:', label_ids: [4], estimate_minutes: 240 }) },
  },
];

// The views the migration seeds, with icons and the groupings the screens expect, plus one for the week ahead.
const view = (id: number, name: string, layout: Layout, group_by: FieldRef | null, filter: Filter | null, icon: string, entity: ViewEntity = 'issue'): SavedView => ({
  id, name, entity, position: id - 1, icon: `icon:${icon}`, color: null,
  def: withDisplay({ filter, group_by, layout }), template_id: null, folder_id: null,
});
const category = (op: 'In' | 'Neq', value: Value): Filter => ({ Cond: { field: { Fixed: 'StatusCategory' }, op, value } });
const views: SavedView[] = [
  view(1, 'Active', 'List', { Fixed: 'Status' }, category('In', ['unstarted', 'started']), 'list'),
  view(2, 'Board', 'Board', { Fixed: 'Status' }, category('In', ['unstarted', 'started']), 'board'),
  view(3, 'Timeline: all projects', 'Gantt', { Fixed: 'Project' }, category('Neq', 'completed'), 'timeline'),
  // shortcut: the 7 days are fixed when the data is made; saved filters have no relative dates yet.
  view(4, 'This week', 'List', null, { And: [category('In', ['unstarted', 'started']), { Cond: { field: { Fixed: 'DueDate' }, op: 'Lte', value: day(7) } }] }, 'calendar'),
  // Two project views; their fields are project fields (the devserver seed makes the same two).
  view(5, 'Project board', 'Board', { Fixed: 'Status' }, null, 'board', 'project'),
  view(6, 'Project timeline', 'Gantt', null, { Cond: { field: { Fixed: 'StatusCategory' }, op: 'NotIn', value: ['completed', 'cancelled'] } }, 'timeline', 'project'),
];

export const fixtures: MockSeed = {
  statuses, projects, propertyDefs, issues, labels, cycles, views, milestones, relations, templates,
  viewTemplates: seedViewTemplates(propertyDefs.map((d) => d.key), day(7)),
  workspace: { setup_done: true, name: 'Cairn', key: 'ABC', timezone: 'UTC' }, history,
};
