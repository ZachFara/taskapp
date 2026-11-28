import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import BlockedCard from './BlockedCard.svelte';
import CycleCard from './CycleCard.svelte';
import FlowCard from './FlowCard.svelte';
import type { Metrics } from './metrics';

globalThis.ResizeObserver ??= class { observe() {} unobserve() {} disconnect() {} } as never;

const base = { from: '2026-11-01', to: '2026-11-07', bucket: 'Day', buckets: [], thin: false } as unknown as Metrics;
let app: ReturnType<typeof mount> | null = null;
function show(component: typeof CycleCard, metrics: Metrics, measure: 'Count' | 'Points' = 'Count') {
  app = mount(component, { target: document.body, props: { metrics, measure } });
  flushSync();
  return document.body.textContent ?? '';
}
afterEach(() => { if (app) unmount(app); app = null; document.body.innerHTML = ''; });

const cycle = { committed: 2, committed_points: 5, done: 1, done_points: 3, say_do_pct: 50, say_do_points_pct: 60, added: 1, added_points: 5, removed: 1, removed_points: 2, scope_change_pct: 100, scope_change_points_pct: 140 };

describe('CycleCard', () => {
  it('shows say/do and scope change in the chosen measure', () => {
    expect(show(CycleCard, { ...base, cycle })).toContain('Say/do50%');
    unmount(app!); document.body.innerHTML = '';
    const text = show(CycleCard, { ...base, cycle }, 'Points');
    expect(text).toContain('Say/do60%');
    expect(text).toContain('Scope change140%');
  });
  it('asks for a cycle when the scope is a range, and notes an empty commitment', () => {
    expect(show(CycleCard, { ...base, cycle: null })).toContain('Choose a cycle');
    unmount(app!); document.body.innerHTML = '';
    expect(show(CycleCard, { ...base, cycle: { ...cycle, committed: 0, say_do_pct: null, scope_change_pct: null } })).toContain('Nothing was in this cycle');
  });
});

describe('BlockedCard', () => {
  it('lists what is blocked now and the average blocked time', () => {
    const blocked = { now: [{ issue_id: 2, key: 'ABC-2', title: 'Ship it', blocked_by: ['ABC-4'], days: 5 }], time: [{ issue_id: 3, key: 'ABC-3', title: 'Old', days: 3 }], avg_days: 1.5 };
    const text = show(BlockedCard, { ...base, blocked });
    expect(text).toContain('Blocked now1');
    expect(text).toContain('1.5 d');
    expect(text).toContain('ABC-2 Ship it');
    expect(text).toContain('ABC-4');
  });
  it('says so when nothing is blocked', () => {
    expect(show(BlockedCard, { ...base, blocked: { now: [], time: [], avg_days: null } })).toContain('Nothing is waiting');
  });
});

describe('FlowCard', () => {
  it('shows the average, the waiting statuses and the per-issue wait', () => {
    const flow = { avg_pct: 47.5, n: 2, distribution: [0, 1, 0, 1, 0], waiting: [{ status_id: 6, name: 'Review', days: 5, issues: 2 }] };
    const text = show(FlowCard, { ...base, flow });
    expect(text).toContain('48%');
    expect(text).toContain('Review');
    expect(text).toContain('2.5 d');
  });
  it('shows the Waiting hint instead of a bare 100% when no time sat waiting', () => {
    const text = show(FlowCard, { ...base, flow: { avg_pct: 100, n: 2, distribution: [0, 0, 0, 0, 2], waiting: [] } });
    expect(text).not.toContain('100%');
    expect(text).toContain('Mark statuses as Waiting');
  });
  it('is blank without a finished issue that has a cycle time', () => {
    expect(show(FlowCard, { ...base, flow: { avg_pct: null, n: 0, distribution: [0, 0, 0, 0, 0], waiting: [] } })).toContain('blank');
  });
});
