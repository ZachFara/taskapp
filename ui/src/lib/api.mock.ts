import type { Api } from './api';
import type {
  Bar, Change, Comment, Cycle, FieldRef, Filter, Issue, Label, Layout, Milestone, Pin, Project, PropertyDef,
  Relation, Reschedule, SavedView, Status, FixedField, Value, ViewDef, WorkBlock,
} from './bindings';

export type MockSeed = {
  statuses: Status[];
  projects: Project[];
  propertyDefs: PropertyDef[];
  issues: Issue[];
  labels?: Label[];
  cycles?: Cycle[];
  views?: SavedView[];
};

const nullableIssueFields = [
  'project_id', 'milestone_id', 'parent_id', 'cycle_id', 'start_date', 'due', 'estimate_minutes',
] as const;
const fail = (kind: string, message: string): never => {
  throw { kind, message };
};
const addDays = (date: string, n: number) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + n);
  return d.toISOString().slice(0, 10);
};
// shortcut: a timed due date counts as its UTC day, upgrade when views filter by local day
const dueDay = (i: Issue) => i.due?.Date ?? i.due?.DateTime?.utc.slice(0, 10) ?? null;
const nothing = (_?: unknown) => null;
const dayDiff = (a: string, b: string) => (Date.parse(b) - Date.parse(a)) / 86_400_000;

export function createMockApi(seed: MockSeed = fixtures): Api {
  const s = structuredClone(seed);
  const labels = s.labels ?? [];
  const cycles = s.cycles ?? [];
  const views = s.views ?? [];
  const milestones: Milestone[] = [];
  const comments: Comment[] = [];
  const relations: Relation[] = [];
  const pins: Pin[] = [];
  const blocks: WorkBlock[] = [];
  const changes: Change[] = [];
  // Undo for issues only: each write keeps the issues as they were before it.
  type Step = { label: string; issues: Issue[] };
  const undoSteps: Step[] = [];
  const redoSteps: Step[] = [];
  const step = <T>(label: string, run: () => T): T => {
    undoSteps.push({ label, issues: structuredClone(s.issues) });
    redoSteps.length = 0;
    return run();
  };
  const swap = (from: Step[], to: Step[], what: string) => {
    const prev = from.pop() ?? fail('Invalid', `nothing to ${what}`);
    to.push({ label: prev.label, issues: structuredClone(s.issues) });
    s.issues.splice(0, s.issues.length, ...prev.issues);
    return { undo: undoSteps.at(-1)?.label ?? null, redo: redoSteps.at(-1)?.label ?? null };
  };
  const settings = new Map<string, Value>([['issue_prefix', 'ZF']]);
  const now = () => new Date().toISOString();
  const next = (rows: { id: number }[]) => Math.max(0, ...rows.map((r) => r.id)) + 1;
  const find = <T extends { id: number }>(rows: T[], id: number, what: string) =>
    rows.find((r) => r.id === id) ?? fail('NotFound', `${what} ${id}`);
  const remove = <T extends { id: number }>(rows: T[], id: number, what: string) =>
    void rows.splice(rows.indexOf(find(rows, id, what)), 1);
  const category = (i: Issue) => s.statuses.find((st) => st.id === i.status_id)?.category;

  function fieldValue(i: Issue, f: FieldRef): Value {
    const prop = f.Property;
    if (prop !== undefined) {
      if (!s.propertyDefs.some((d) => d.key === prop)) fail('Invalid', `unknown property ${prop}`);
      return i.properties[prop] ?? null;
    }
    const by: Record<FixedField, Value> = {
      Title: i.title, Status: i.status_id, StatusCategory: category(i) ?? null, Priority: i.priority,
      Project: i.project_id, Milestone: i.milestone_id, Cycle: i.cycle_id, Label: i.label_ids,
      Parent: i.parent_id, StartDate: i.start_date, DueDate: dueDay(i), Estimate: i.estimate_minutes,
      CreatedAt: i.created_at, UpdatedAt: i.updated_at, CompletedAt: i.completed_at,
      Archived: i.archived_at !== null,
    };
    return by[f.Fixed!];
  }

  const isEmpty = (v: Value) => v === null || v === '' || (Array.isArray(v) && v.length === 0);
  const eq = (have: Value, want: Value) =>
    Array.isArray(have) && !Array.isArray(want) ? have.includes(want) : have === want;
  const holds = (i: Issue, f: Filter): boolean => {
    if (f.And) return f.And.every((x) => holds(i, x));
    if (f.Or) return f.Or.some((x) => holds(i, x));
    if (f.Not) return !holds(i, f.Not);
    const { field, op, value: want } = f.Cond!;
    const have = fieldValue(i, field);
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
  const query = (def: ViewDef) => {
    const rows = s.issues.filter((i) => (def.include_archived || !i.archived_at) && (!def.filter || holds(i, def.filter)));
    for (const { field, descending } of [...def.sorts].reverse()) {
      // nulls sort last in either direction
      rows.sort((a, b) => {
        const [x, y] = [fieldValue(a, field), fieldValue(b, field)];
        if (x === y) return 0;
        if (x === null) return 1;
        if (y === null) return -1;
        return (x < y ? -1 : 1) * (descending ? -1 : 1);
      });
    }
    return rows;
  };

  const viewIssues = (id: number) => query(find(views, id, 'view').def);
  const create = (input: Parameters<Api['issues']['create']>[0]): Issue => {
    if (!input.title?.trim()) fail('Invalid', 'title is empty');
    const number = Math.max(0, ...s.issues.map((i) => i.id)) + 1;
    const at = now();
    const issue: Issue = {
      id: number, key: `${settings.get('issue_prefix')}-${number}`, title: input.title!.trim(),
      description: input.description ?? '',
      status_id: input.status_id ?? [...s.statuses].sort((a, b) => a.position - b.position)[0].id,
      priority: input.priority ?? 0, project_id: input.project_id ?? null, milestone_id: input.milestone_id ?? null,
      parent_id: input.parent_id ?? null, cycle_id: input.cycle_id ?? null, label_ids: input.label_ids ?? [],
      start_date: input.start_date ?? null, due: input.due ?? null, estimate_minutes: input.estimate_minutes ?? null,
      properties: input.properties ?? {}, created_at: input.created_at ?? at, updated_at: at,
      completed_at: input.completed_at ?? null, archived_at: null,
    };
    s.issues.push(issue);
    return issue;
  };
  const getIssue = (id: number) => find(s.issues, id, 'issue');
  const parse = (input: string) => {
    const priority = /(?:^|\s)!([0-4])(?=\s|$)/.exec(input);
    const project = /(?:^|\s)#([\w-]+)/.exec(input);
    return {
      title: input.replace(/(?:^|\s)[!#][\w-]+/g, '').trim(), due: null, start_date: null,
      priority: priority ? Number(priority[1]) : null, project_slug: project?.[1] ?? null,
      label_names: [], estimate_minutes: null,
    };
  };
  const shift = (moved: Reschedule, seen = new Set<number>()): Reschedule[] => {
    const out: Reschedule[] = [];
    for (const r of relations.filter((r) => r.kind === 'Blocks' && r.from === moved.issue_id)) {
      const dep = getIssue(r.to);
      if (seen.has(dep.id) || !dep.start_date || dep.start_date > moved.due_date) continue;
      seen.add(dep.id);
      const by = dayDiff(dep.start_date, addDays(moved.due_date, 1));
      const next = { issue_id: dep.id, start_date: addDays(dep.start_date, by), due_date: addDays(dueDay(dep) ?? dep.start_date, by) };
      out.push(next, ...shift(next, seen));
    }
    return out;
  };
  const upsert = <T extends { id: number }>(rows: T[], row: T, what: string) =>
    Object.assign(find(rows, row.id, what), row);
  const def = (key: string) => s.propertyDefs.find((d) => d.key === key) ?? fail('NotFound', `property ${key}`);
  const backups: string[] = [];
  const ok = async () => null;
  // Sets position to each row's index in `order`; like the real commands, the order must list every row once.
  const reposition = <T extends { position: number }, K>(rows: T[], order: K[], keyOf: (r: T) => K, what: string) => {
    if (order.length !== rows.length || rows.some((r) => order.filter((k) => k === keyOf(r)).length !== 1))
      fail('Invalid', `reorder must list every ${what} exactly once`);
    for (const r of rows) r.position = order.indexOf(keyOf(r));
  };
  const off = { reminders: 'Off', gcal: 'Off', export: 'Off', backup: 'Off' } as const;

  return {
    issues: {
      create: async (input) => step('issues_create', () => create(input)),
      update: async (id, patch) => step('issues_update', () => {
        const i = getIssue(id);
        for (const [k, v] of Object.entries(patch) as [keyof Issue & string, never][]) {
          if (v === undefined || (v === null && !(nullableIssueFields as readonly string[]).includes(k))) continue;
          changes.push({ id: next(changes), at: now(), source: 'Ui', entity: 'issue', entity_id: String(id), field: k, old_value: i[k as keyof Issue] as Value, new_value: v });
          (i as Record<string, unknown>)[k] = v;
        }
        i.updated_at = now();
        i.completed_at = category(i) === 'Completed' ? (i.completed_at ?? i.updated_at) : null;
        return i;
      }),
      get: async (id) => getIssue(id),
      getByKey: async (key) => s.issues.find((i) => i.key === key) ?? fail('NotFound', `issue ${key}`),
      archive: async (id) => step('issues_archive', () => nothing((getIssue(id).archived_at = now()))),
      restore: async (id) => step('issues_restore', () => nothing((getIssue(id).archived_at = null))),
      children: async (id) => s.issues.filter((i) => i.parent_id === id),
      parseQuickadd: async (input) => parse(input),
      createFromQuickadd: async (input) => {
        const p = parse(input);
        return create({ title: p.title, priority: p.priority, project_id: s.projects.find((x) => x.slug === p.project_slug)?.id });
      },
    },
    statuses: {
      list: async () => [...s.statuses].sort((a, b) => a.position - b.position),
      create: async (name, category, color, icon) => {
        const st = { id: next(s.statuses), name, category, color, color_hex: color, icon: icon ?? null, position: s.statuses.length };
        s.statuses.push(st);
        return st;
      },
      update: async (id, name, color, position, icon) => {
        const st = find(s.statuses, id, 'status');
        return Object.assign(st, { name: name ?? st.name, color: color ?? st.color, position: position ?? st.position, icon: icon ?? st.icon });
      },
      delete: async (id, moveIssuesTo) => {
        find(s.statuses, moveIssuesTo, 'status');
        s.issues.forEach((i) => i.status_id === id && (i.status_id = moveIssuesTo));
        return nothing(remove(s.statuses, id, 'status'));
      },
    },
    properties: {
      listDefs: async () => [...s.propertyDefs].sort((a, b) => a.position - b.position),
      createDef: async (d) => {
        if (s.propertyDefs.some((x) => x.key === d.key)) fail('Conflict', `property ${d.key} exists`);
        s.propertyDefs.push(d);
        return d;
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
        for (const i of s.issues) if (kind === 'Number' && i.properties[key] != null) i.properties[key] = Number(i.properties[key]);
        return Object.assign(d, { kind, options });
      },
      reorder: async (keys) => {
        reposition(s.propertyDefs, keys, (d) => d.key, 'property');
        return null;
      },
      setScope: async (key, project) => {
        const d = def(key);
        const bad = s.issues.filter((i) => i.properties[key] != null && project !== null && i.project_id !== project);
        if (bad.length) fail('Conflict', `${key} has values on issues outside the project: ${bad.map((i) => i.key).join(', ')}`);
        return Object.assign(d, { project_id: project });
      },
      deleteDef: async (key) => {
        s.propertyDefs.splice(s.propertyDefs.indexOf(def(key)), 1);
        s.issues.forEach((i) => delete i.properties[key]);
        return null;
      },
      setValue: async (issue, key, value) => {
        def(key);
        const i = getIssue(issue);
        if (value === null) delete i.properties[key];
        else i.properties[key] = value;
        i.updated_at = now();
        return i;
      },
    },
    projects: {
      list: async () => s.projects,
      create: async (slug, name, icon, color) => {
        if (s.projects.some((p) => p.slug === slug)) fail('Conflict', `project ${slug} exists`);
        const p = { id: next(s.projects), slug, name, description: null, doc_link: null, start_date: null, target_date: null, archived_at: null, icon: icon ?? null, color: color ?? null };
        s.projects.push(p);
        return p;
      },
      update: async (p) => upsert(s.projects, p, 'project'),
      delete: async (id) => {
        remove(s.projects, id, 'project');
        s.issues.forEach((i) => i.project_id === id && (i.project_id = null));
        return null;
      },
      progress: async (id) => {
        const rows = s.issues.filter((i) => i.project_id === id);
        return [rows.filter((i) => category(i) === 'Completed').length, rows.length];
      },
      listMilestones: async (project) => milestones.filter((m) => m.project_id === project),
      createMilestone: async (project, name, targetDate) => {
        const m = { id: next(milestones), project_id: find(s.projects, project, 'project').id, name, target_date: targetDate };
        milestones.push(m);
        return m;
      },
      updateMilestone: async (m) => upsert(milestones, m, 'milestone'),
      deleteMilestone: async (id) => nothing(remove(milestones, id, 'milestone')),
    },
    cycles: {
      list: async () => cycles,
      create: async (name, startDate, endDate, icon, color) => {
        const c = { id: next(cycles), number: Math.max(0, ...cycles.map((x) => x.number)) + 1, name, start_date: startDate, end_date: endDate, icon: icon ?? null, color: color ?? null };
        cycles.push(c);
        return c;
      },
      update: async (c) => upsert(cycles, c, 'cycle'),
      delete: async (id) => nothing(remove(cycles, id, 'cycle')),
    },
    labels: {
      list: async () => labels,
      create: async (name, color, icon) => {
        if (labels.some((l) => l.name === name)) fail('Conflict', `label ${name} exists`);
        const l = { id: next(labels), name, color, color_hex: color, icon: icon ?? null };
        labels.push(l);
        return l;
      },
      update: async (l) => upsert(labels, l, 'label'),
      delete: async (id) => {
        remove(labels, id, 'label');
        s.issues.forEach((i) => (i.label_ids = i.label_ids.filter((x) => x !== id)));
        return null;
      },
      addLabel: async (issue, label) => {
        find(labels, label, 'label');
        const i = getIssue(issue);
        if (!i.label_ids.includes(label)) i.label_ids.push(label);
        return null;
      },
      removeLabel: async (issue, label) => {
        const i = getIssue(issue);
        i.label_ids = i.label_ids.filter((x) => x !== label);
        return null;
      },
    },
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
      add: async (rel) => {
        getIssue(rel.from);
        getIssue(rel.to);
        if (!relations.some((r) => r.from === rel.from && r.to === rel.to && r.kind === rel.kind)) relations.push(rel);
        return null;
      },
      remove: async (rel) => {
        const at = relations.findIndex((r) => r.from === rel.from && r.to === rel.to && r.kind === rel.kind);
        if (at >= 0) relations.splice(at, 1);
        return null;
      },
      forIssue: async (id) => relations.filter((r) => r.from === id || r.to === id),
    },
    schedule: {
      bars: async (viewId) =>
        viewIssues(viewId).flatMap((i): Bar[] => {
          const due = dueDay(i);
          if (i.start_date && due) return [{ issue_id: i.id, start: i.start_date, end: due, from_estimate: false }];
          if (i.start_date && i.estimate_minutes) {
            // 1 day = 480 minutes
            return [{ issue_id: i.id, start: i.start_date, end: addDays(i.start_date, Math.ceil(i.estimate_minutes / 480)), from_estimate: true }];
          }
          return [];
        }),
      criticalPath: async (viewId) => {
        const ids = new Set(viewIssues(viewId).map((i) => i.id));
        const longest = (id: number): number[] => {
          const tails = relations.filter((r) => r.kind === 'Blocks' && r.from === id && ids.has(r.to)).map((r) => longest(r.to));
          return [id, ...tails.reduce((a, b) => (b.length > a.length ? b : a), [])];
        };
        return [...ids].map(longest).reduce((a, b) => (b.length > a.length ? b : a), []);
      },
      shiftDependents: async (moved) => {
        const moves = shift(moved);
        for (const m of moves) Object.assign(getIssue(m.issue_id), { start_date: m.start_date, due: { Date: m.due_date } });
        return moves;
      },
    },
    views: {
      query: async (d) => query(d),
      list: async () => views,
      save: async (id, name, d, icon, color) => {
        if (id === null) {
          const v = { id: next(views), name, def: d, position: views.length, icon: icon ?? null, color: color ?? null };
          views.push(v);
          return v;
        }
        const v = find(views, id, 'view');
        return Object.assign(v, { name, def: d, icon: icon ?? v.icon, color: color ?? v.color });
      },
      delete: async (id) => nothing(remove(views, id, 'view')),
      reorder: async (ids) => {
        reposition(views, ids, (v) => v.id, 'view');
        views.sort((a, b) => a.position - b.position);
        return null;
      },
    },
    search: {
      search: async (text, limit) => {
        const t = text.toLowerCase();
        return s.issues.filter((i) => `${i.title} ${i.description}`.toLowerCase().includes(t)).slice(0, limit);
      },
    },
    pins: {
      pin: async (kind, target, title) => {
        const have = pins.find((p) => p.kind === kind && p.target === target);
        if (have) return have;
        const p = { id: next(pins), kind, target, title, position: pins.length };
        pins.push(p);
        return p;
      },
      unpin: async (id) => nothing(remove(pins, id, 'pin')),
      reorder: async (ids) => {
        ids.forEach((id, position) => (find(pins, id, 'pin').position = position));
        return null;
      },
      listResolved: async () =>
        [...pins].sort((a, b) => a.position - b.position).map((pin) => {
          const route = { Issue: `/issue/${pin.target}`, Project: `/project/${pin.target}`, View: `/view/${pin.target}`, Cycle: '/cycles', Note: pin.target, Url: pin.target }[pin.kind];
          return { pin, label: pin.title ?? pin.target, archived: false, route };
        }),
    },
    blocks: {
      create: async (issue, start, end) => {
        const b = { id: next(blocks), issue_id: getIssue(issue).id, start_utc: start, end_utc: end };
        blocks.push(b);
        return b;
      },
      move: async (id, start, end) => Object.assign(find(blocks, id, 'block'), { start_utc: start, end_utc: end }),
      delete: async (id) => nothing(remove(blocks, id, 'block')),
      between: async (from, to) => blocks.filter((b) => b.start_utc < to && b.end_utc > from),
    },
    history: {
      listFor: async (entity, entityId) => changes.filter((c) => c.entity === entity && c.entity_id === entityId),
      undo: async () => swap(undoSteps, redoSteps, 'undo'),
      redo: async () => swap(redoSteps, undoSteps, 'redo'),
      undoStatus: async () => ({ undo: undoSteps.at(-1)?.label ?? null, redo: redoSteps.at(-1)?.label ?? null }),
    },
    settings: {
      get: async (key) => settings.get(key) ?? null,
      set: async (key, value) => nothing(settings.set(key, value)),
    },
    backup: {
      snapshot: async (dir, reason) => {
        const path = `${dir}/snapshots/mock-${reason}.db`;
        backups.unshift(path);
        return path;
      },
      list: async (dir) => backups.filter((p) => p.startsWith(`${dir}/`)),
    },
    export: { now: ok },
    sync: { status: async () => off, now: async () => off },
    gcal: { connect: ok, listCalendars: async () => [['primary', 'Primary']] },
    reminders: { listLists: async () => [['1', 'Reminders']] },
  };
}

const statuses: Status[] = (
  [['Backlog', 'Backlog'], ['Todo', 'Unstarted'], ['In Progress', 'Started'], ['Done', 'Completed'], ['Canceled', 'Canceled']] as const
).map(([name, category], position) => ({ id: position + 1, name, category, position, color: 'gray', color_hex: '#6b7280', icon: null }));

const projects: Project[] = [
  { id: 1, slug: 'home', name: 'Home', description: null, doc_link: null, start_date: null, target_date: null, archived_at: null, icon: null, color: null },
  { id: 2, slug: 'work', name: 'Work', description: null, doc_link: null, start_date: null, target_date: null, archived_at: null, icon: null, color: null },
];

const propertyDefs: PropertyDef[] = [
  { key: 'area', name: 'Area', kind: 'Select', options: [{ value: 'home', color: '#4caf50' }, { value: 'work', color: '#2196f3' }], project_id: null, position: 0 },
  { key: 'effort', name: 'Effort', kind: 'Number', options: [], project_id: null, position: 1 },
  { key: 'review_on', name: 'Review on', kind: 'Date', options: [], project_id: null, position: 2 },
];

const issues: Issue[] = Array.from({ length: 20 }, (_, k) => {
  const n = k + 1;
  const created = `2026-11-${String(n).padStart(2, '0')}T09:00:00Z`;
  return {
    id: n, key: `ZF-${n}`, title: `Fixture task ${n}`, description: '', status_id: (k % 5) + 1, priority: n % 5,
    project_id: n % 3 === 0 ? null : (n % 2) + 1, milestone_id: null, parent_id: null, cycle_id: null, label_ids: [],
    start_date: n % 4 === 0 ? `2026-12-${String(n).padStart(2, '0')}` : null,
    due: n % 4 === 0 ? { Date: `2026-12-${String(n + 2).padStart(2, '0')}` } : null,
    estimate_minutes: n % 2 ? 480 : null,
    properties: (n % 3 === 1 ? { area: 'home', effort: n } : n % 3 === 2 ? { area: 'work' } : {}) as Issue['properties'],
    created_at: created, updated_at: created, completed_at: null, archived_at: null,
  };
});

// Same three views the migration seeds. shortcut: no status filter, the mock query has no StatusCategory support.
const view = (id: number, name: string, layout: Layout, group_by: FieldRef | null = null): SavedView => ({
  id, name, position: id - 1,
  def: { filter: null, sorts: [], group_by, layout, columns: [], include_archived: false },
});
const views: SavedView[] = [
  view(1, 'Active', 'List'),
  view(2, 'Board', 'Board', { Fixed: 'Status' }),
  view(3, 'Timeline: all projects', 'Gantt', { Fixed: 'Project' }),
];

export const fixtures: MockSeed = { statuses, projects, propertyDefs, issues, views };
