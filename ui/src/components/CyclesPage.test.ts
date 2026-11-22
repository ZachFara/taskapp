import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import type { Cycle } from '../lib/bindings';
import CyclesPage from './CyclesPage.svelte';

// Explicit "today" so the current, upcoming and past split does not drift with the calendar.
const TODAY = new Date('2026-11-22T12:00:00Z');
const cycle = (id: number, start: string, end: string): Cycle => ({ id, number: id, name: `Cycle ${id}`, start_date: start, end_date: end, icon: null, color: null });
const cycles = [cycle(1, '2026-10-01', '2026-10-14'), cycle(2, '2026-11-16', '2026-11-29'), cycle(3, '2026-12-01', '2026-12-14')];

let live: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(TODAY);
  history.replaceState(null, '', '/cycles');
});
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.useRealTimers();
  vi.restoreAllMocks();
});

async function open(api: Api) {
  setApi(api);
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(CyclesPage, { target });
  await vi.waitFor(() => expect(target.querySelector('h1')).not.toBeNull());
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
  return target;
}
const button = (root: ParentNode, text: string) => [...root.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === text)!;
const group = (t: HTMLElement, name: string) => t.querySelector<HTMLElement>(`[data-group="${name}"]`);

describe('CyclesPage', () => {
  it('group headers carry a cycle icon, never a status glyph', async () => {
    const target = await open(createMockApi({ ...fixtures, cycles }));
    for (const band of ['current', 'upcoming', 'past']) {
      const header = group(target, band)!;
      expect(header.querySelector('[data-category]'), band).toBeNull();
      expect(header.querySelector('[data-icon="cycle"]'), band).not.toBeNull();
    }
  });

  it('cycles_grouped_current_upcoming_past', async () => {
    const target = await open(createMockApi({ ...fixtures, cycles }));
    expect(group(target, 'current')?.textContent).toContain('Cycle 2');
    expect(group(target, 'upcoming')?.textContent).toContain('Cycle 3');
    // Past is collapsed by default: its header counts it, its rows appear on expand.
    expect(group(target, 'past')?.textContent).toContain('1');
    expect(group(target, 'past')?.textContent).not.toContain('Cycle 1');
    button(group(target, 'past')!, 'Past 1').click();
    flushSync();
    expect(group(target, 'past')?.textContent).toContain('Cycle 1');
    expect(group(target, 'current')?.textContent).not.toContain('Cycle 1');
  });

  it('move_unfinished_is_one_bulk_update', async () => {
    const seed = { ...fixtures, cycles, issues: fixtures.issues.map((i, n) => (n < 3 ? { ...i, cycle_id: 2 } : i)) };
    const mock = createMockApi(seed);
    const unfinished = (await mock.views.query({ filter: { Cond: { field: { Fixed: 'Cycle' }, op: 'Eq', value: 2 } }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }))
      .filter((i) => !['Completed', 'Canceled'].includes(seed.statuses.find((s) => s.id === i.status_id)?.category ?? ''));
    // The mock has no bulk update yet; this stand-in applies the patch per id and records the single call.
    const bulk = vi.fn(async (ids: number[], p: { patch?: { cycle_id?: number | null } }) => {
      for (const id of ids) await mock.issues.update(id, p.patch ?? {});
      return null;
    });
    (mock.issues as unknown as { bulkUpdate: typeof bulk }).bulkUpdate = bulk;
    const target = await open(mock);
    button(target, 'Move unfinished to next cycle').click();
    flushSync();
    button(target.ownerDocument, 'Move').click();
    await vi.waitFor(() => expect(bulk).toHaveBeenCalledTimes(1));
    expect(bulk).toHaveBeenCalledWith(unfinished.map((i) => i.id), { patch: { cycle_id: 3 } });
  });

  it('rows_show_human_dates_and_days_left', async () => {
    const target = await open(createMockApi({ ...fixtures, cycles }));
    expect(group(target, 'current')?.textContent).toContain('Nov 16 to Nov 29');
    expect(group(target, 'current')?.textContent).toContain('7 days left');
  });

  it('empty_cycles_shows_designed_empty_state', async () => {
    const target = await open(createMockApi({ ...fixtures, cycles: [] }));
    expect(target.textContent).toContain('No cycles yet');
  });

  it('the command menu opens the move and new-cycle dialogs', async () => {
    const target = await open(createMockApi(fixtures));
    window.dispatchEvent(new CustomEvent('command:cycle.move_unfinished'));
    flushSync();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('Move unfinished issues');
    window.dispatchEvent(new CustomEvent('command:cycle.new'));
    flushSync();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('New cycle');
    expect(target).toBeTruthy();
  });
});
