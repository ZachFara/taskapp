import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import type { Issue, ViewDef } from '../../lib/bindings';
import ListView from './ListView.svelte';

const groupBy: ViewDef = { filter: null, sorts: [], group_by: { Fixed: 'Status' }, layout: 'List', columns: [], include_archived: false };
const viewSeed = (def: Partial<ViewDef> = {}) => [{ id: 1, name: 'All', def: { ...groupBy, ...def }, position: 0 }];

const issue = (n: number, over: Partial<Issue> = {}): Issue => ({
  ...fixtures.issues[0], id: n, key: `ZF-${n}`, title: `Row ${n}`, status_id: 1, priority: 0, project_id: null,
  start_date: null, due: null, estimate_minutes: null, properties: {}, ...over,
});

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  vi.useRealTimers();
});

async function open(seed: MockSeed) {
  const mock = createMockApi(seed);
  setApi(mock);
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(ListView, { target, props: { view: seed.views![0] } });
  flushSync();
  return { mock, target };
}
const rows = (t: HTMLElement) => [...t.querySelectorAll<HTMLElement>('[role="option"]')];
const heads = (t: HTMLElement) => [...t.querySelectorAll<HTMLElement>('.group-head')].map((h) => h.textContent?.trim());
const press = (key: string) => window.dispatchEvent(new KeyboardEvent('keydown', { key }));
const selected = (t: HTMLElement) => rows(t).find((r) => r.getAttribute('aria-selected') === 'true')?.dataset.key;

describe('ListView', () => {
  it('renders_groups_in_status_order', async () => {
    const { target } = await open({ ...fixtures, views: viewSeed() });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    expect(heads(target)).toEqual(['Backlog', 'Todo', 'In Progress', 'Done', 'Canceled']);
  });

  it('jk_moves_selection', async () => {
    const { target } = await open({ ...fixtures, views: viewSeed() });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    const first = selected(target);
    press('j');
    flushSync();
    expect(selected(target)).not.toBe(first);
    press('k');
    flushSync();
    expect(selected(target)).toBe(first);
  });

  it('j at the last row stays on the last row', async () => {
    const { target } = await open({ ...fixtures, views: viewSeed() });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    const last = rows(target).at(-1)!.dataset.key;
    for (let i = 0; i < rows(target).length + 3; i++) press('j');
    flushSync();
    expect(selected(target)).toBe(last);
  });

  it('s_changes_status_via_api', async () => {
    const { mock, target } = await open({ ...fixtures, views: viewSeed() });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    const spy = vi.spyOn(mock.issues, 'update');
    const id = Number(selected(target)!.replace('ZF-', ''));
    press('s');
    flushSync();
    [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Done')!.click();
    await vi.waitFor(() => expect(spy).toHaveBeenCalledWith(id, { status_id: 4 }));
  });

  it('p_changes_priority_via_api', async () => {
    const { mock, target } = await open({ ...fixtures, views: viewSeed() });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    const spy = vi.spyOn(mock.issues, 'update');
    const id = Number(selected(target)!.replace('ZF-', ''));
    press('p');
    flushSync();
    [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Urgent')!.click();
    await vi.waitFor(() => expect(spy).toHaveBeenCalledWith(id, { priority: 1 }));
  });

  it('enter_opens_issue_detail', async () => {
    const { target } = await open({ ...fixtures, views: viewSeed() });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    const key = selected(target);
    press('Enter');
    expect(location.pathname).toBe(`/issue/${key}`);
  });

  it('renders_1000_issues_under_100ms', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] });
    const many = Array.from({ length: 1000 }, (_, k) => issue(k + 1));
    const started = performance.now();
    const { target } = await open({ ...fixtures, issues: many, views: viewSeed({ group_by: null }) });
    await vi.waitFor(() => expect(rows(target).length).toBeGreaterThan(0));
    expect(performance.now() - started).toBeLessThan(100);
    expect(rows(target).length).toBeLessThan(100);
    expect(target.textContent).toContain('Row 1');
    expect(target.textContent).not.toContain('Row 1000');
  });

  it('empty view shows an empty state', async () => {
    const { target } = await open({ ...fixtures, issues: [], views: viewSeed() });
    await vi.waitFor(() => expect(target.querySelector('.empty')).not.toBeNull());
    expect(rows(target)).toHaveLength(0);
  });

  it('unknown status id still renders under Unknown', async () => {
    const seed = { ...fixtures, issues: [issue(1, { status_id: 99 })], views: viewSeed() };
    const { target } = await open(seed);
    await vi.waitFor(() => expect(rows(target).length).toBe(1));
    expect(heads(target)).toEqual(['Unknown']);
    expect(target.textContent).toContain('Row 1');
  });

  it('row shows due date and configured property chips', async () => {
    const seed = {
      ...fixtures,
      issues: [issue(1, { due: { Date: '2026-12-03' }, properties: { area: 'home' } })],
      views: viewSeed({ columns: [{ Property: 'area' }] }),
    };
    const { target } = await open(seed);
    await vi.waitFor(() => expect(rows(target).length).toBe(1));
    expect(rows(target)[0].textContent).toContain('2026-12-03');
    expect(rows(target)[0].textContent).toContain('home');
  });
});
