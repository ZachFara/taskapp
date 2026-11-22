import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import type { Issue, SavedView, ViewDef } from '../../lib/bindings';
import BoardView from './BoardView.svelte';
import ListView from './ListView.svelte';
import TableView from './TableView.svelte';

const issue = (n: number, points: number): Issue => ({ ...fixtures.issues[0], id: n, key: `ABC-${n}`, title: `Row ${n}`, points, properties: {} });
const def = (layout: ViewDef['layout']): ViewDef => ({
  filter: null, sorts: [], group_by: { Fixed: 'Points' }, layout, columns: [{ Fixed: 'Title' }, { Fixed: 'Points' }], include_archived: false,
});
let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});

async function text(Comp: typeof ListView, layout: ViewDef['layout'], estimation: string) {
  const view: SavedView = { id: 9, name: 'V', def: def(layout), position: 0, icon: null, color: null };
  const mock = createMockApi({ ...fixtures, issues: [issue(1, 5), issue(2, 8)], views: [view] } as MockSeed);
  await mock.settings.set('estimation', estimation);
  setApi(mock);
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(Comp, { target, props: { view } });
  flushSync();
  await vi.waitFor(() => expect(target.textContent).toContain('ABC-1'));
  await new Promise((r) => setTimeout(r, 30));
  flushSync();
  return target.textContent ?? '';
}
import { vi } from 'vitest';

describe('points off', () => {
  for (const [name, Comp, layout] of [['list', ListView, 'List'], ['table', TableView, 'Table'], ['board', BoardView, 'Board']] as const) {
    it(`${name}_ignores_a_stored_points_group_and_column_when_estimation_is_none_or_time`, async () => {
      for (const est of ['none', 'time']) {
        const t = await text(Comp as typeof ListView, layout, est);
        expect(t, est).not.toMatch(/\d points|No points|Points/);
        unmount(live!);
        live = null;
        document.body.innerHTML = '';
      }
    });
    it(`${name}_still_shows_points_when_on`, async () => {
      expect(await text(Comp as typeof ListView, layout, 'points')).toMatch(/5 points|Points/);
    });
  }
});
