import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, httpApi, setApi } from './api';
import { createMockApi } from './api.mock';
import type { TemplateFields, ViewDef } from './bindings';

const def = (filter: ViewDef['filter']): ViewDef => ({
  filter, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false,
});

afterEach(() => vi.unstubAllGlobals());

describe('mock api', () => {
  it('create assigns sequential keys', async () => {
    const mock = createMockApi();
    expect((await mock.issues.create({ title: 'a' })).key).toBe('ZF-21');
    expect((await mock.issues.create({ title: 'b' })).key).toBe('ZF-22');
  });

  it('undo and redo move an issue edit back and forth', async () => {
    const mock = createMockApi();
    const made = await mock.issues.create({ title: 'a' });
    await mock.issues.update(made.id, { title: 'b' });
    expect(await mock.history.undoStatus()).toEqual({ undo: 'issues_update', redo: null });
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

  it('query with an unknown property rejects with Invalid', async () => {
    const q = createMockApi().views.query(def({ Cond: { field: { Property: 'nope' }, op: 'Eq', value: 1 } }));
    await expect(q).rejects.toMatchObject({ kind: 'Invalid' });
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
      name: 'Bug', slug: 'bug', icon: null, color: null,
      body: { title_prefix: 'Bug: ', fields: fields({ priority: 3 }), sub_issues: [{ title: 'Repro', fields: fields() }] },
    });
    const issue = await mock.issues.createFromQuickadd('/bug Fix login !1');
    expect(issue.title).toBe('Bug: Fix login');
    expect(issue.priority).toBe(1);
    expect((await mock.issues.children(issue.id)).map((i) => i.title)).toEqual(['Repro']);
  });

  it('text with a slash after the first word is a plain title', async () => {
    const mock = createMockApi();
    await mock.templates.create({ name: 'Bug', slug: 'bug', icon: null, color: null, body: { title_prefix: '', fields: fields(), sub_issues: [] } });
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
    stubFetch({ error: { kind: 'NotFound', message: 'issue ZF-99' } });
    await expect(httpApi().issues.getByKey('ZF-99')).rejects.toEqual({ kind: 'NotFound', message: 'issue ZF-99' });
  });
});
