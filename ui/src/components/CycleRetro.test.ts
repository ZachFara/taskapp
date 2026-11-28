import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import { listCommands } from '../lib/commands';
import type { Cycle } from '../lib/bindings';
import type { Metrics } from './dashboard/metrics';
import CycleRetro from './CycleRetro.svelte';
import CyclesPage from './CyclesPage.svelte';

const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
proto.scrollIntoView ??= () => {};
globalThis.ResizeObserver ??= class { observe() {} unobserve() {} disconnect() {} } as never;

const cycle: Cycle = { id: 1, number: 1, name: 'Sprint', start_date: '2026-10-01', end_date: '2026-10-14', icon: null, color: null };
const metrics = {
  from: cycle.start_date, to: cycle.end_date, bucket: 'Day', buckets: [], lead_time: { p50_days: null, p85_days: null, n: 0 },
  cycle_time: { p50_days: null, p85_days: null, n: 0, points: [] }, wip_now: 0, by_label: [], by_source: [], velocity: [], thin: true,
  cycle: { committed: 4, committed_points: 0, done: 3, done_points: 0, say_do_pct: 75, say_do_points_pct: null, added: 1, added_points: 0, removed: 0, removed_points: 0, scope_change_pct: 25, scope_change_points_pct: null },
} as unknown as Metrics;

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  history.replaceState(null, '', '/');
});
const settle = async () => {
  for (let i = 0; i < 12; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
function api() {
  const a = createMockApi({ ...fixtures, cycles: [cycle] }) as unknown as Api & { metrics: { for: () => Promise<Metrics> } };
  a.metrics.for = async () => metrics;
  setApi(a);
  return a;
}
const field = <T extends Element>(sel: string) => document.body.querySelector<T>(sel)!;

describe('cycle retrospective', () => {
  it('saves the goal, the yes/no and the rating, and shows the cycle numbers', async () => {
    const a = api();
    live = mount(CycleRetro, { target: document.body, props: { cycle: (await a.cycles.list())[0] } });
    await settle();
    expect(document.body.textContent).toContain('75%'); // say/do from the cycle scope
    const goal = field<HTMLInputElement>('input[aria-label="Cycle goal"]');
    goal.value = 'Ship it';
    goal.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    expect((await a.cycles.list())[0].goal).toBe('Ship it');
    const four = [...document.body.querySelectorAll<HTMLElement>('[aria-label="Rating"] [role="tab"]')].find((t) => t.textContent?.trim() === '4')!;
    four.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }));
    four.click();
    await settle();
    expect((await a.cycles.list())[0].rating).toBe(4);
  });

  it('the goal, met and rating commands act on the open cycle', async () => {
    const a = api();
    const ctx = { route: '/cycles', selection: [], focusedIssueId: null, openIssueId: null, api: a, navigate: () => {} };
    expect(listCommands(ctx).map((c) => c.id)).toEqual(expect.arrayContaining(['cycle.goal', 'cycle.goal_met', 'cycle.rating', 'cycle.retro']));
    history.replaceState(null, '', '/cycles#1');
    const page = (id: string) => listCommands(ctx).find((c) => c.id === id)!.page!(ctx);
    await (await Promise.resolve(page('cycle.goal_met').items) as { id: string; run: () => Promise<void> }[]).find((i) => i.id === 'no')!.run();
    await page('cycle.goal').parse!('Learn the codebase')!.run();
    await (page('cycle.rating').items as { id: string; run: () => Promise<void> }[]).find((i) => i.id === '2')!.run();
    expect((await a.cycles.list())[0]).toMatchObject({ goal_met: false, goal: 'Learn the codebase', rating: 2 });
  });

  it('opens from /cycles#number', async () => {
    api();
    history.replaceState(null, '', '/cycles#1');
    live = mount(CyclesPage, { target: document.body });
    await vi.waitFor(() => expect(document.body.querySelector('[aria-label="Retrospective"]')).not.toBeNull());
  });
});
