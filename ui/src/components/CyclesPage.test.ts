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

  it('move_unfinished_updates_each_open_issue_once', async () => {
    const seed = { ...fixtures, cycles, issues: fixtures.issues.map((i, n) => (n < 3 ? { ...i, cycle_id: 2 } : i)) };
    const mock = createMockApi(seed);
    const unfinished = (await mock.views.query({ filter: { Cond: { field: { Fixed: 'Cycle' }, op: 'Eq', value: 2 } }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }))
      .filter((i) => !['Completed', 'Canceled'].includes(seed.statuses.find((s) => s.id === i.status_id)?.category ?? ''));
    const update = vi.spyOn(mock.issues, 'update');
    const target = await open(mock);
    button(target, 'Move unfinished to next cycle').click();
    flushSync();
    button(target.ownerDocument, 'Move').click();
    await vi.waitFor(() => expect(update).toHaveBeenCalledTimes(unfinished.length));
    for (const [id, patch] of update.mock.calls) {
      expect(unfinished.map((i) => i.id)).toContain(id);
      expect(patch).toEqual({ cycle_id: 3 });
    }
  });

  it('empty_cycles_shows_designed_empty_state', async () => {
    const target = await open(createMockApi({ ...fixtures, cycles: [] }));
    expect(target.textContent).toContain('No cycles yet');
  });
});
