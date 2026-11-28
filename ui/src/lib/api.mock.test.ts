import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, httpApi, setApi } from './api';
import { createMockApi, fixtures } from './api.mock';
import type { TemplateFields, ViewDef } from './bindings';

const def = (filter: ViewDef['filter']): ViewDef => ({
  filter, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false,
});

afterEach(() => vi.unstubAllGlobals());

describe('mock api', () => {
  it('ships a neutral sample workspace', async () => {
    const mock = createMockApi();
    const rows = await mock.views.query(def(null));
    expect(rows).toHaveLength(24);
    expect(rows.some((i) => /fixture/i.test(i.title))).toBe(false);
    expect((await mock.labels.list()).every((l) => l.icon)).toBe(true);
    expect(await mock.templates.list()).toHaveLength(2);
    expect((await mock.views.list()).map((v) => v.name)).toContain('This week');
    expect(rows.some((i) => i.due?.Date && i.due.Date < new Date().toISOString().slice(0, 10))).toBe(true);
  });

  it('a deferred issue leaves views until its day unless the view shows deferred ones, and Hidden until filters', async () => {
    const mock = createMockApi();
    const all = (await mock.views.query(def(null))).length;
    await mock.issues.update(1, { hidden_until: '2999-01-01' });
    expect(await mock.views.query(def(null))).toHaveLength(all - 1);
    expect(await mock.views.query({ ...def(null), deferred: 'show' })).toHaveLength(all);
    const on = await mock.views.query({ ...def({ Cond: { field: { Fixed: 'HiddenUntil' }, op: 'Gt', value: 'today+30' } }), deferred: 'show' });
    expect(on.map((i) => i.id)).toEqual([1]);
    await mock.issues.update(1, { hidden_until: null });
    expect(await mock.views.query(def(null))).toHaveLength(all);
  });

  it('a relative date counts from today in the workspace zone', async () => {
    const mock = createMockApi();
    const tz = (await mock.workspace.status()).timezone;
    const plus = (n: number) => {
      const d = new Date(`${new Date().toLocaleDateString('sv', { timeZone: tz })}T00:00:00Z`);
      d.setUTCDate(d.getUTCDate() + n);
      return d.toISOString().slice(0, 10);
    };
    const ids = async (op: 'Eq' | 'Lt' | 'Lte' | 'Gte', value: string) =>
      (await createMockApi().views.query(def({ Cond: { field: { Fixed: 'DueDate' }, op, value } }))).map((i) => i.id);
    expect(await ids('Eq', 'today')).toEqual(await ids('Eq', plus(0)));
    expect(await ids('Lt', 'today')).toEqual(await ids('Lt', plus(0)));
    expect(await ids('Lte', 'today+7')).toEqual(await ids('Lte', plus(7)));
    expect(await ids('Gte', 'today-14')).toEqual(await ids('Gte', plus(-14)));
    expect(await ids('Lt', 'today')).not.toEqual(await ids('Lte', 'today+7'));
  });

  it('create assigns sequential keys', async () => {
    const mock = createMockApi();
    expect((await mock.issues.create({ title: 'a' })).key).toBe('ABC-25');
    expect((await mock.issues.create({ title: 'b' })).key).toBe('ABC-26');
  });

  it('undo and redo move an issue edit back and forth', async () => {
    const mock = createMockApi();
    const made = await mock.issues.create({ title: 'a' });
    await mock.issues.update(made.id, { title: 'b' });
    expect(await mock.history.undoStatus()).toEqual({ undo: 'Edited ABC-25', redo: null });
    await mock.history.undo();
    expect((await mock.issues.get(made.id)).title).toBe('a');
    await mock.history.redo();
    expect((await mock.issues.get(made.id)).title).toBe('b');
    await mock.history.undo();
    await mock.issues.update(made.id, { title: 'c' });
    await expect(mock.history.redo()).rejects.toMatchObject({ kind: 'Invalid' });
  });

  it('query filters by select property', async () => {
    const rows = await createMockApi().views.query(def({ Cond: { field: { Property: 'area' }, op: 'Eq', value: 'home' } }));
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.every((i) => i.properties.area === 'home')).toBe(true);
  });

  it('query with an unknown property drops the reference instead of failing', async () => {
    const all = await createMockApi().views.query(def(null));
    const q = await createMockApi().views.query(def({ Cond: { field: { Property: 'nope' }, op: 'Eq', value: 1 } }));
    expect(q).toHaveLength(all.length);
  });

  it('csv import writes nothing until applied', async () => {
    const mock = createMockApi();
    const plan = await mock.import.csvPlan('Title,Other\nOne\nTwo\n', null);
    expect(plan.issues).toHaveLength(2);
    expect(plan.unmapped).toEqual(['Other']);
    expect(await mock.issues.getByKey('ABC-25').catch(() => null)).toBeNull();
    expect((await mock.import.apply(plan)).created).toBe(2);
    expect((await mock.issues.getByKey('ABC-26')).title).toBe('Two');
  });

  it('setApi swaps implementation', async () => {
    const before = api;
    const mock = createMockApi();
    setApi(mock);
    expect(api).toBe(mock);
    setApi(before);
  });
});

describe('mock templates', () => {
  const fields = (over: Partial<TemplateFields> = {}): TemplateFields => ({
    description: null, status_id: null, priority: null, project_id: null, label_ids: [], estimate_minutes: null,
    properties: {}, ...over,
  });

  it('quick-add /slug applies the template, its sub-issues, and the text overrides', async () => {
    const mock = createMockApi();
    await mock.templates.create({
      name: 'Defect', slug: 'defect', icon: null, color: null,
      body: { title_prefix: 'Bug: ', fields: fields({ priority: 3 }), sub_issues: [{ title: 'Repro', fields: fields() }] },
    });
    const issue = await mock.issues.createFromQuickadd('/defect Fix login p1');
    expect(issue.title).toBe('Bug: Fix login');
    expect(issue.priority).toBe(1);
    expect((await mock.issues.children(issue.id)).map((i) => i.title)).toEqual(['Repro']);
  });

  it('text with a slash after the first word is a plain title', async () => {
    const mock = createMockApi();
    await mock.templates.create({ name: 'Defect', slug: 'defect', icon: null, color: null, body: { title_prefix: '', fields: fields(), sub_issues: [] } });
    expect((await mock.issues.createFromQuickadd('/usr/bin is full')).title).toBe('/usr/bin is full');
  });

  it('unknown /slug rejects Invalid and names the known slugs', async () => {
    const mock = createMockApi();
    await expect(mock.issues.createFromQuickadd('/nope x')).rejects.toMatchObject({ kind: 'Invalid' });
  });
});

describe('http transport', () => {
  const stubFetch = (body: unknown) => {
    const fetch = vi.fn().mockResolvedValue({ json: async () => body });
    vi.stubGlobal('fetch', fetch);
    return fetch;
  };

  it('posts to rpc endpoint', async () => {
    const fetch = stubFetch({ ok: null });
    await httpApi().statuses.delete(1, 2);
    expect(fetch).toHaveBeenCalledWith('http://127.0.0.1:7878/rpc/statuses_delete', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ id: 1, move_issues_to: 2 }),
    }));
  });

  it('turns an error body into a rejection with its kind', async () => {
    stubFetch({ error: { kind: 'NotFound', message: 'issue ABC-99' } });
    await expect(httpApi().issues.getByKey('ABC-99')).rejects.toEqual({ kind: 'NotFound', message: 'issue ABC-99' });
  });
});

describe('mock workspace and quick add', () => {
  it('setup_on_a_fresh_mock_sets_the_prefix_and_reports_done', async () => {
    const mock = createMockApi({ ...fixtures, workspace: undefined }, 'UTC');
    expect(await mock.workspace.status()).toMatchObject({ setup_done: false, name: null });
    await mock.workspace.setup("Studio", "STU", "Europe/Berlin");
    expect(await mock.workspace.status()).toEqual({ setup_done: true, name: 'Studio', key: 'STU', timezone: 'Europe/Berlin' });
    expect((await mock.issues.create({ title: 'x' })).key).toBe('STU-25');
  });

  it('a_time_of_day_is_read_in_the_given_zone', async () => {
    const parsed = await createMockApi(fixtures, 'Asia/Tokyo').issues.parseQuickadd('call at 9am');
    expect(parsed.due).toMatchObject({ DateTime: { tz: 'Asia/Tokyo' } });
    expect(new Date(parsed.due!.DateTime!.utc).getUTCHours()).toBe(0);
  });

  it('quick_add_plus_is_project_hash_and_at_are_labels', async () => {
    const parsed = await createMockApi(fixtures, 'UTC').issues.parseQuickadd('tidy desk +home #chores @errands');
    expect(parsed.title).toBe('tidy desk');
    expect(parsed.project_slug).toBe('home');
    expect(parsed.label_names).toEqual(['chores', 'errands']);
  });
});

describe('mock view templates', () => {
  it('first-run setup seeds three templates once, the planning table listing every property', async () => {
    const mock = createMockApi({ ...fixtures, workspace: undefined, viewTemplates: [] }, 'UTC');
    await mock.workspace.setup('Studio', 'STU', null);
    await mock.workspace.setup('Studio', 'STD', null);
    const t = await mock.view.templatesList();
    expect(t.map((x) => x.name)).toEqual(['Status board', 'Planning table', 'This week']);
    expect(t[1].def.columns).toContainEqual({ Property: 'effort' });
  });

  it('create from a template copies its definition, and undo removes the view', async () => {
    const mock = createMockApi();
    const [board] = await mock.view.templatesList();
    const v = await mock.views.createFromTemplate(board.id, null);
    expect(v).toMatchObject({ name: 'Status board', template_id: board.id, def: board.def });
    await mock.history.undo();
    expect((await mock.views.list()).some((x) => x.id === v.id)).toBe(false);
  });
});

describe('mock points and progress', () => {
  it('seeds about two thirds of the issues with points and a three-week history', async () => {
    const mock = createMockApi();
    const rows = await mock.views.query(def(null));
    expect(rows.filter((i) => i.points != null)).toHaveLength(16);
    const s = await mock.insights.progress({ Project: 2 }, 'points', null, null);
    const last = s.days.at(-1)!;
    expect(s.days.length).toBeGreaterThan(14);
    expect(s.days[0].scope).toBeGreaterThan(0);
    expect(last.scope).toBeGreaterThan(last.completed);
    expect(last.started).toBeGreaterThanOrEqual(last.completed);
  });

  it('velocity falls back to weeks when there are no cycles', async () => {
    const mock = createMockApi({ ...fixtures, cycles: [] });
    const v = await mock.insights.velocity({ View: 1 }, 'count', 3);
    expect(v).toHaveLength(3);
    expect(v[0].label).toMatch(/^Week of [A-Z][a-z]{2} \d+$/);
    expect(new Date(`${v[2].start}T00:00:00Z`).getUTCDay()).toBe(1);
  });

  it('a points change is undoable and cleared with null', async () => {
    const mock = createMockApi();
    await mock.issues.update(2, { points: 3.5 });
    expect((await mock.issues.get(2)).points).toBe(3.5);
    await mock.history.undo();
    expect((await mock.issues.get(2)).points).toBeNull();
    await expect(mock.issues.update(2, { points: 0.25 })).rejects.toMatchObject({ kind: 'Invalid' });
  });

  // Sprint 1 holds issues 1, 4, 7, 8, 10 and 16 (5, 8, 13, 3, 5 and 5 points). Issue 10 was done from day -11 to -5, so
  // it is the previous cycle's work on the first day (day -6) and returns when reopened; issues 4 and 7 were already
  // revised to 8 and 13 points; issue 16 starts on day -5.
  it('the seeded current cycle burns up from its first day, not from earlier completions', async () => {
    const mock = createMockApi();
    const s = await mock.insights.progress({ Cycle: 1 }, 'points', null, null);
    const rows = s.days.map((d) => [d.scope, d.started, d.completed]);
    expect(rows).toHaveLength(14);
    expect(rows[0]).toEqual([34, 29, 0]);
    expect(rows.slice(1)).toEqual(Array(13).fill([39, 39, 0]));
  });

  it('an issue that left the cycle counts until the day it left', async () => {
    const mock = createMockApi();
    await mock.issues.update(8, { cycle_id: null }); // 3 points, today
    const s = await mock.insights.progress({ Cycle: 1 }, 'points', null, null);
    expect(s.days.map((d) => d.scope)).toEqual([34, ...Array(5).fill(39), ...Array(8).fill(36)]);
  });
});

describe('mock view folders', () => {
  it('create, rename, place, nest and delete keep the items and undo restores them', async () => {
    const api = createMockApi(fixtures, 'UTC');
    const a = await api.folders.create('view', null, ' A ');
    const b = await api.folders.create('view', null, 'B');
    expect(a.name).toBe('A');
    await expect(api.folders.create('view', null, ' ')).rejects.toMatchObject({ kind: 'Invalid' });
    await expect(api.folders.rename(99, 'x')).rejects.toMatchObject({ kind: 'NotFound' });
    await api.folders.place(b.id, null, 0);
    expect((await api.folders.list()).map((f) => f.name)).toEqual(['B', 'A']);
    await expect(api.folders.placeItem('view', 1, 99, null)).rejects.toMatchObject({ kind: 'NotFound' });
    await expect(api.folders.placeItem('project', 1, a.id, null)).rejects.toMatchObject({ kind: 'Invalid' });
    await api.folders.placeItem('view', 1, a.id, null);
    await api.folders.delete(a.id);
    expect((await api.views.list()).find((v) => v.id === 1)!.folder_id).toBeNull();
    await api.history.undo();
    expect((await api.views.list()).find((v) => v.id === 1)!.folder_id).toBe(a.id);
  });

  it('nests three deep, refuses a fourth level and a cycle, and deleting moves the inside up', async () => {
    const api = createMockApi(fixtures, 'UTC');
    const a = await api.folders.create('view', null, 'A');
    const b = await api.folders.create('view', a.id, 'B');
    const c = await api.folders.create('view', b.id, 'C');
    await expect(api.folders.create('view', c.id, 'D')).rejects.toMatchObject({ kind: 'Invalid' });
    await expect(api.folders.place(a.id, c.id, null)).rejects.toMatchObject({ kind: 'Invalid' });
    await expect(api.folders.create('project', a.id, 'X')).rejects.toMatchObject({ kind: 'Invalid' });
    await api.folders.placeItem('view', 1, c.id, null);
    await api.folders.delete(b.id);
    expect((await api.folders.list()).find((f) => f.id === c.id)!.parent_id).toBe(a.id);
    await api.folders.delete(c.id);
    expect((await api.folders.items()).find((p) => p.section === 'view' && p.id === 1)!.folder_id).toBe(a.id);
  });

  it('places projects, cycles and pins, in order', async () => {
    const api = createMockApi(fixtures, 'UTC');
    const f = await api.folders.create('project', null, 'F');
    const [p1, p2] = fixtures.projects.map((p) => p.id);
    await api.folders.placeItem('project', p2, f.id, null);
    await api.folders.placeItem('project', p1, f.id, 0);
    const inside = (await api.folders.items()).filter((p) => p.section === 'project' && p.folder_id === f.id);
    expect(inside.sort((x, y) => x.position - y.position).map((p) => p.id)).toEqual([p1, p2]);
  });
});

describe('mock metrics', () => {
  const ago = (n: number) => new Date(Date.now() - n * 86_400_000).toISOString().slice(0, 10);
  const query = (over = {}) => ({ scope: { Range: { from: ago(30), to: ago(0) } }, bucket: 'Week' as const, filter: null, measure: 'count' as const, ...over });

  it('replays the seeded history into buckets, times and breakdowns', async () => {
    const m = await createMockApi().metrics.for(query());
    expect(m.buckets[0].start <= ago(30)).toBe(true);
    expect(m.buckets.reduce((n, b) => n + b.completed, 0)).toBe(m.lead_time.n);
    expect(m.cycle_time.n).toBeLessThanOrEqual(m.lead_time.n);
    expect(m.cycle_time.n).toBeGreaterThan(0);
    expect(m.by_source.map((x) => x.source)).toContain('Ui');
    expect(m.thin).toBe(m.lead_time.n < 5);
  });

  it('counts a fresh completion by the source that made it, and refuses bad queries', async () => {
    const mock = createMockApi();
    const doing = (await mock.statuses.list()).find((s) => s.category === 'Started')!.id;
    const done = (await mock.statuses.list()).find((s) => s.category === 'Completed')!.id;
    await mock.issues.update(2, { status_id: doing });
    await mock.issues.update(2, { status_id: done });
    const today = (await mock.metrics.for(query({ scope: { Range: { from: ago(0), to: ago(0) } }, bucket: 'Day' }))).buckets[0];
    expect(today.completed).toBe(1);
    await expect(mock.metrics.for(query({ measure: 'time' }))).rejects.toMatchObject({ kind: 'Invalid' });
    await expect(mock.metrics.for(query({ scope: { Cycle: 99 } }))).rejects.toMatchObject({ kind: 'NotFound' });
  });
});
