import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import type { SavedView, ViewDef } from '../lib/bindings';
import { listCommands, shortcutFor } from '../lib/commands';
import { refreshAll } from '../lib/stores.svelte';
import ViewPage from './ViewPage.svelte';

// Popovers and user-event are slow on a loaded machine.
vi.setConfig({ testTimeout: 30000 });

const def = (over: Partial<ViewDef> = {}): ViewDef => ({
  filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false, ...over,
});

let mock: Api;
// Creates its own statuses and issues so no fixture string or id is assumed.
async function open(over: Partial<ViewDef> = {}, name = 'Mine') {
  const v: SavedView = { id: 1, name, def: def(over), position: 0, icon: null, color: null };
  mock = createMockApi({ ...fixtures, views: [v] });
  setApi(mock);
  const sts = await mock.statuses.list();
  const open = sts.find((s) => s.category === 'Started')!;
  const done = sts.find((s) => s.category === 'Completed')!;
  await mock.issues.create({ title: 'Alpha task', status_id: open.id } as never);
  await mock.issues.create({ title: 'Beta task', status_id: done.id } as never);
  await refreshAll();
  const r = render(ViewPage, { params: { id: '1' } });
  await screen.findByRole('heading', { level: 1 });
  return r;
}
const saved = async () => (await mock.views.list())[0];
afterEach(cleanup);

describe('ViewPage', () => {
  it('switching_layout_renders_it_and_persists', async () => {
    const user = userEvent.setup();
    const { container } = await open();
    expect(container.querySelector('[data-column]')).toBeNull();
    await user.click(screen.getByRole('tab', { name: /Board/ }));
    await waitFor(() => expect(container.querySelector('[data-column]')).not.toBeNull());
    await waitFor(async () => expect((await saved()).def.layout).toBe('Board'), { timeout: 2000 });
  });

  it('v_then_a_letter_switches_layout', async () => {
    const { container } = await open();
    const ctx = { route: '/view/1', selection: [], focusedIssueId: null, openIssueId: null, api: mock, navigate: () => {} };
    const layouts = ['List', 'Board', 'Table', 'Gantt', 'Calendar'];
    const cmds = listCommands(ctx)
      .filter((c) => c.id.startsWith('view.layout.'))
      .sort((a, b) => String(a.shortcut).localeCompare(String(b.shortcut)));
    expect(cmds.map((c) => shortcutFor(c.id))).toEqual(['v b', 'v c', 'v g', 'v l', 'v t']);
    await cmds[0].run!(ctx); // v b: board
    await waitFor(() => expect(container.querySelector('[data-column]')).not.toBeNull());
    await waitFor(async () => expect((await saved()).def.layout).toBe(layouts[1]), { timeout: 2000 });
  });

  it('status_category_filter_narrows_results', async () => {
    const user = userEvent.setup();
    await open();
    await screen.findByText('Alpha task');
    await user.click(screen.getByRole('button', { name: 'Add filter' }));
    await user.click(await screen.findByText('Status category'));
    await user.click(await screen.findByLabelText('Completed'));
    await waitFor(() => expect(screen.queryByText('Alpha task')).toBeNull());
    expect(screen.getByText('Beta task')).toBeTruthy();
    await waitFor(async () => expect((await saved()).def.filter).toEqual({
      Cond: { field: { Fixed: 'StatusCategory' }, op: 'In', value: ['completed'] },
    }), { timeout: 2000 });
  });

  it('a_saved_contract_shaped_filter_shows_as_a_chip_and_removes', async () => {
    const user = userEvent.setup();
    await open({ filter: { And: [{ Cond: { field: { Fixed: 'StatusCategory' }, op: 'In', value: ['completed'] } }] } });
    await waitFor(() => expect(screen.queryByText('Alpha task')).toBeNull());
    expect(screen.getByRole('button', { name: /^Status category is/ })).toBeTruthy();
    await user.click(screen.getByRole('button', { name: 'Remove filter Status category' }));
    await screen.findByText('Alpha task');
    await waitFor(async () => expect((await saved()).def.filter).toBeNull(), { timeout: 2000 });
  });

  it('quick_filter_narrows_without_saving_until_saved', async () => {
    const user = userEvent.setup();
    await open();
    await screen.findByText('Beta task');
    await user.click(screen.getByRole('button', { name: 'Quick filter' }));
    await user.type(await screen.findByLabelText('Quick filter text'), 'Alpha');
    await waitFor(() => expect(screen.queryByText('Beta task')).toBeNull());
    expect(screen.getByText('Temporary')).toBeTruthy();
    await new Promise((r) => setTimeout(r, 600));
    expect((await saved()).def.filter).toBeNull();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.getByText('Beta task')).toBeTruthy());
    await user.click(screen.getByRole('button', { name: 'Quick filter' }));
    await user.type(await screen.findByLabelText('Quick filter text'), 'Alpha');
    await user.click(await screen.findByRole('button', { name: 'Save to view' }));
    await waitFor(async () => expect((await saved()).def.filter).toEqual({
      Cond: { field: { Fixed: 'Title' }, op: 'Contains', value: 'Alpha' },
    }), { timeout: 2000 });
    expect(await screen.findByRole('button', { name: /^Title contains Alpha/ })).toBeTruthy(); // the kept filter is now a chip
  });

  it('empty_result_offers_clear_filters', async () => {
    const user = userEvent.setup();
    await open({ filter: { Cond: { field: { Fixed: 'Title' }, op: 'Contains', value: 'no such title' } } });
    await user.click(await screen.findByRole('button', { name: 'Clear filters' }));
    await screen.findByText('Alpha task');
  });

  it('rename_persists', async () => {
    const user = userEvent.setup();
    await open();
    await user.click(screen.getByRole('button', { name: 'Rename view' }));
    const input = await screen.findByLabelText('View name');
    await user.clear(input);
    await user.type(input, 'Renamed{Enter}');
    await waitFor(async () => expect((await saved()).name).toBe('Renamed'), { timeout: 2000 });
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Renamed');
  });

  it('pin_and_delete', async () => {
    const user = userEvent.setup();
    await open();
    await user.click(screen.getByRole('button', { name: 'Pin view' }));
    await waitFor(async () => expect((await mock.pins.listResolved()).length).toBe(1));
    await user.click(screen.getByRole('button', { name: 'More' }));
    await user.click(await screen.findByText('Delete view'));
    await user.click(await screen.findByRole('button', { name: 'Delete' }));
    await waitFor(async () => expect(await mock.views.list()).toEqual([]));
  });

  it('save_as_new_view_adds_a_copy', async () => {
    const user = userEvent.setup();
    await open();
    await user.click(screen.getByRole('button', { name: 'More' }));
    await user.click(await screen.findByText('Save as new view'));
    await waitFor(async () => expect((await mock.views.list()).length).toBe(2));
  });
});
