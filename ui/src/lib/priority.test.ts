import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from './api.mock';
import { setApi } from './api';
import type { Issue, SavedView, ViewDef } from './bindings';
import { refreshAll } from './stores.svelte';
import { listCommands, type Command, type Ctx } from './commands';
import { makeLookup } from '../components/filters/lookup';
import ListView from '../components/views/ListView.svelte';
import TableView from '../components/views/TableView.svelte';
import BoardView from '../components/views/BoardView.svelte';

// Backend: 0 none, 1 urgent, 2 high, 3 medium, 4 low. The glyph counts the other way (4 rings = urgent).
const TABLE: [number, string, number][] = [[0, 'No priority', 0], [1, 'Urgent', 4], [2, 'High', 3], [3, 'Medium', 2], [4, 'Low', 1]];
const [S1] = fixtures.statuses;
const issue = (n: number, priority: number): Issue => ({
  ...fixtures.issues[0], id: n, key: `ABC-${n}`, title: `Row ${n}`, status_id: S1.id, priority, project_id: null,
  cycle_id: null, label_ids: [], start_date: null, due: null, estimate_minutes: null, properties: {},
});
const issues = TABLE.map(([p]) => issue(p + 1, p));
const def = (layout: ViewDef['layout'], group_by: ViewDef['group_by'] = null): SavedView => ({
  id: 71, name: 'V', position: 0, icon: null, color: null,
  def: { filter: null, sorts: [], group_by, layout, columns: [], include_archived: false },
});

let live: ReturnType<typeof mount> | null = null;
afterEach(() => { if (live) unmount(live); live = null; document.body.innerHTML = ''; });

async function render(comp: typeof ListView | typeof TableView | typeof BoardView, view: SavedView, ready: string) {
  setApi(createMockApi({ ...fixtures, issues, views: [view] } as MockSeed));
  await refreshAll();
  const target = document.body.appendChild(document.createElement('div'));
  live = mount(comp as typeof ListView, { target, props: { view } });
  flushSync();
  await new Promise((r) => setTimeout(r, 20));
  flushSync();
  expect(target.querySelector(ready)).not.toBeNull();
  return target;
}
// The glyph of issue ABC-n inside whatever element holds it.
const glyphOf = (t: HTMLElement, key: string) => {
  const g = t.querySelector(`[data-key="${key}"] [role=img][data-level], [data-issue="${key}"] [role=img][data-level]`)!;
  return [g.getAttribute('aria-label'), Number(g.getAttribute('data-level'))];
};

describe('priority means the same thing on every screen', () => {
  it.each([['List', ListView, 'li[data-key]'], ['Table', TableView, 'tr[data-key]'], ['Board', BoardView, '[data-issue]']] as const)(
    '%s renders glyph and name for 0-4', async (layout, comp, ready) => {
      const t = await render(comp, def(layout), ready);
      for (const [p, name, level] of TABLE) expect(glyphOf(t, `ABC-${p + 1}`), `${layout} p${p}`).toEqual([name, level]);
    });

  it.each([['List', ListView, 'li[data-key]'], ['Table', TableView, 'tr[data-key]']] as const)(
    '%s groups by priority with names in urgent-first order, none last', async (layout, comp, ready) => {
      const t = await render(comp, def(layout, { Fixed: 'Priority' }), ready);
      const names = [...t.querySelectorAll('.band .name, .group .name, [data-group] .name')].map((n) => n.textContent?.trim());
      expect(names).toEqual(['Urgent', 'High', 'Medium', 'Low', 'No priority']);
    });

  it('the p command saves the backend number for each name', async () => {
    const api = createMockApi();
    const bulk = vi.fn(async () => null);
    (api.issues as unknown as { bulkUpdate: unknown }).bulkUpdate = bulk;
    const ctx = { route: '/', selection: [1], focusedIssueId: null, openIssueId: null, api, navigate: () => {} } as unknown as Ctx;
    const cmd = listCommands({} as Ctx, { all: true }).find((c) => c.id === 'issue.priority') as Command;
    const saved: Record<string, number> = {};
    for (const it of await cmd.page!(ctx).items) {
      bulk.mockClear();
      await it.run();
      saved[it.label] = (bulk.mock.calls[0] as unknown as [number[], { patch: { priority: number } }])[1].patch.priority;
    }
    expect(saved).toEqual({ 'No priority': 0, Urgent: 1, High: 2, Medium: 3, Low: 4 });
  });

  it('the filter lookup offers the same names for the same numbers', () => {
    const l = makeLookup({ statuses: [], projects: [], cycles: [], labels: [], defs: [], now: new Date() } as never);
    expect(l.choices({ Fixed: 'Priority' }).map((c) => [c.value, c.label])).toEqual(TABLE.map(([p, n]) => [p, n]));
  });
});
