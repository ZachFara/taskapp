import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { createMockApi, fixtures, type MockSeed } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import { loadFields } from '../../lib/fields.svelte';
import { refreshAll } from '../../lib/stores.svelte';
import type { Issue, SavedView, ViewDef } from '../../lib/bindings';
import BoardView from '../views/BoardView.svelte';
import ListView from '../views/ListView.svelte';
import TableView from '../views/TableView.svelte';
import DisplayPanel from './DisplayPanel.svelte';

const issue = (n: number, points: number | null, status = fixtures.statuses[0].id): Issue => ({
  ...fixtures.issues[0], id: n, key: `ABC-${n}`, title: `Row ${n}`, status_id: status, priority: 0, project_id: null, cycle_id: null,
  label_ids: [], start_date: null, due: null, estimate_minutes: null, parent_id: null, properties: {}, points,
});
const def = (over: Record<string, unknown> = {}, layout: ViewDef['layout'] = 'List'): ViewDef =>
  ({ filter: null, sorts: [], group_by: null, layout, columns: [], include_archived: false, ...over }) as ViewDef;
const viewOf = (d: ViewDef): SavedView => ({ id: 5, name: 'V', def: d, position: 0, icon: null, color: null, template_id: null });
const ISSUES = [issue(1, 3), issue(2, 2), issue(3, null)];

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});
async function render(comp: Parameters<typeof mount>[0], props: Record<string, unknown>, ready: string, estimation = 'both') {
  const api = createMockApi({ ...fixtures, issues: ISSUES, views: [] } as MockSeed);
  await api.settings.set('estimation', estimation);
  setApi(api);
  await loadFields();
  await refreshAll();
  const target = document.body.appendChild(document.createElement('div'));
  live = mount(comp, { target, props });
  flushSync();
  await vi.waitFor(() => expect(target.querySelector(ready)).not.toBeNull(), { interval: 1 });
  await new Promise((r) => setTimeout(r, 30));
  flushSync();
  return target;
}
const heads = (t: Element) => [...t.querySelectorAll('.band')].map((b) => b.textContent!.replace(/\s+/g, ' ').trim());

describe('Points in the Display panel', () => {
  const panel = (estimation: string) => render(DisplayPanel, { def: def(), onchange: () => {} }, '[data-field="F:Status"]', estimation);
  it('is offered as a hidden field', async () => {
    expect((await panel('both')).querySelector('[data-section="Hidden"] [data-field="F:Points"]')).not.toBeNull();
  });
  it('stays in the saved columns while its field is off', async () => {
    const onchange = vi.fn();
    const t = await render(DisplayPanel, { def: def({ columns: [{ Fixed: 'Status' }, { Fixed: 'Points' }] }), onchange }, '[data-field="F:Status"]', 'none');
    (t.querySelector('[data-field="F:Status"] input[type="checkbox"], [data-field="F:Status"] [role="switch"]') as HTMLElement).click();
    flushSync();
    expect(onchange).toHaveBeenCalledWith({ columns: [{ Fixed: 'Points' }] });
  });
  it.each(['none', 'time'])('is not offered when estimation is %s', async (e) => {
    expect((await panel(e)).querySelector('[data-field="F:Points"]')).toBeNull();
  });
});

describe('Points as a field', () => {
  const list = (estimation = 'both', over = {}) =>
    render(ListView, { view: viewOf(def({ columns: [{ Fixed: 'Points' }], ...over })) }, '[data-key="ABC-1"], [data-row], li', estimation);
  it('shows a tabular number per row and nothing for no points', async () => {
    const t = await list();
    const row = (k: string) => t.querySelector(`[data-key="${k}"]`)!.textContent!;
    expect(row('ABC-1')).toContain('3');
    expect(t.querySelector('[data-key="ABC-1"] .num')).not.toBeNull();
    expect(t.querySelector('[data-key="ABC-3"] .num')?.textContent ?? '').toBe('');
  });
  it('is hidden when estimation is none or time', async () => {
    for (const e of ['none', 'time']) {
      unmount(live!);
      document.body.innerHTML = '';
      expect((await list(e)).querySelector('[data-key="ABC-1"] .num')).toBeNull();
    }
  });
  it('shows the group total next to the count when the field is shown', async () => {
    const t = await list('both', { group_by: { Fixed: 'Status' } });
    expect(heads(t)[0]).toMatch(/Backlog 3 5 pts/);
  });
  it('shows no total when the field is not shown', async () => {
    const t = await list('both', { group_by: { Fixed: 'Status' }, columns: [{ Fixed: 'Project' }] });
    expect(heads(t)[0]).not.toMatch(/pts/);
  });
  it('groups a list by points', async () => {
    const t = await list('both', { group_by: { Fixed: 'Points' } });
    expect(heads(t).map((h) => h.replace(/ \d+ pts$/, ''))).toEqual(['2 points 1', '3 points 1', 'No points 1']);
  });
  it('groups a table by points and totals each group', async () => {
    const t = await render(TableView, { view: viewOf(def({ group_by: { Fixed: 'Points' }, columns: [{ Fixed: 'Points' }] }, 'Table')) }, 'tbody tr[data-key]');
    expect(heads(t)).toEqual(['2 points 1 2 pts', '3 points 1 3 pts', 'No points 1']);
  });
  it('groups a board by points and totals each column', async () => {
    const t = await render(BoardView, { view: viewOf(def({ group_by: { Fixed: 'Points' }, card_fields: [{ Fixed: 'Points' }] }, 'Board')) }, '.band');
    expect(heads(t)).toEqual(['2 points 1 2 pts', '3 points 1 3 pts', 'No points 1']);
  });
});
