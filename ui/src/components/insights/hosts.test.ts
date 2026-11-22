import { mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ViewPage from '../../app/ViewPage.svelte';
import { setApi, type Api } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import type { Cycle, SavedView } from '../../lib/bindings';
import { refreshAll } from '../../lib/stores.svelte';
import CyclesPage from '../CyclesPage.svelte';
import ProjectPage from '../ProjectPage.svelte';
import { insightsApi, type ProgressSeries } from './insightsApi';
import ProgressSummary from './ProgressSummary.svelte';

const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
proto.scrollIntoView ??= () => {};
vi.setConfig({ testTimeout: 30000 });

const series: ProgressSeries = {
  measure: 'points', from: '2026-11-16', to: '2026-11-20',
  days: ['16', '17', '18', '19', '20'].map((d, i) => ({ date: `2026-11-${d}`, scope: 10, started: i * 2, completed: i })),
};
const flat: ProgressSeries = { ...series, days: series.days.map((d) => ({ ...d, scope: 0 })) };
let api: ReturnType<typeof createMockApi>;
let apps: ReturnType<typeof mount>[] = [];
const wait = () => new Promise((r) => setTimeout(r, 20));
const withSeries = (s: ProgressSeries | null) => insightsApi.setSource({ progress: async () => s, velocity: async () => [] });
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date('2026-11-18T12:00:00Z'));
  api = createMockApi();
  setApi(api as unknown as Api);
  withSeries(series);
});
afterEach(() => {
  apps.splice(0).forEach((a) => void unmount(a));
  document.body.innerHTML = '';
  insightsApi.setSource(null);
  vi.useRealTimers();
});
const put = async (C: never, props: Record<string, unknown>) => {
  const target = document.body.appendChild(document.createElement('div'));
  apps.push(mount(C, { target, props }));
  await vi.waitFor(() => expect(target.textContent).not.toBe(''));
  await wait();
  return target;
};
const toggle = () => dispatchEvent(new CustomEvent('command:insights.toggle'));
const panel = () => document.querySelector('aside[aria-label=Insights]');

describe('ProgressSummary (the project hero)', () => {
  it('draws the compact chart when there is history', async () => {
    const t = await put(ProgressSummary as never, { projectId: 1, progress: [2, 5] });
    expect(t.querySelector('svg[role=img]')).not.toBeNull();
    expect(t.textContent).toContain('2 / 5');
    expect(t.textContent).not.toContain('View as table');
  });
  it('keeps the plain bar when there is no history', async () => {
    withSeries(flat);
    const t = await put(ProgressSummary as never, { projectId: 1, progress: [2, 5] });
    expect(t.querySelector('svg')).toBeNull();
    expect(t.querySelector('[role=img][aria-label="2 of 5 done"]')).not.toBeNull();
  });
  it('keeps the plain bar when the backend fails', async () => {
    insightsApi.setSource({ progress: () => Promise.reject(new Error('down')), velocity: () => Promise.resolve([]) });
    const t = await put(ProgressSummary as never, { projectId: 1, progress: [2, 5] });
    expect(t.querySelector('[role=img][aria-label="2 of 5 done"]')).not.toBeNull();
  });
  it('keeps the plain bar when progress charts are switched off', async () => {
    await api.settings.set('insights_charts', false);
    const t = await put(ProgressSummary as never, { projectId: 1, progress: [2, 5] });
    expect(t.querySelector('svg')).toBeNull();
  });
  it('opens the full panel on demand', async () => {
    const got = vi.fn();
    addEventListener('command:insights.toggle', got);
    const t = await put(ProgressSummary as never, { projectId: 1, progress: [2, 5] });
    [...t.querySelectorAll('button')].find((b) => b.textContent?.includes('Insights'))!.click();
    removeEventListener('command:insights.toggle', got);
    expect(got).toHaveBeenCalledTimes(1);
  });
});

describe('ViewPage', () => {
  const v: SavedView = { id: 1, name: 'Mine', def: { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }, position: 0, icon: null, color: null };
  async function open() {
    api = createMockApi({ ...fixtures, views: [v] });
    setApi(api as unknown as Api);
    await refreshAll();
    return put(ViewPage as never, { params: { id: '1' } });
  }
  it('is closed by default and the command toggles the panel for this view', async () => {
    await open();
    expect(panel()).toBeNull();
    toggle();
    await wait();
    expect(panel()).not.toBeNull();
    toggle();
    await wait();
    expect(panel()).toBeNull();
  });
  it('has a header button that opens it', async () => {
    const t = await open();
    [...t.querySelectorAll('button')].find((b) => b.getAttribute('aria-label') === 'Insights')!.click();
    await wait();
    expect(panel()).not.toBeNull();
  });
});

describe('ProjectPage', () => {
  it('shows the compact chart in the hero and the panel on demand', async () => {
    history.replaceState(null, '', `/project/${fixtures.projects[0].slug}`);
    const t = await put(ProjectPage as never, { params: { slug: fixtures.projects[0].slug } });
    await vi.waitFor(() => expect(t.querySelector('[data-testid=progress] svg[role=img]')).not.toBeNull());
    expect(panel()).toBeNull();
    toggle();
    await wait();
    expect(panel()).not.toBeNull();
  });
});

describe('CyclesPage', () => {
  const cycle = (id: number, start: string, end: string): Cycle => ({ id, number: id, name: `Cycle ${id}`, start_date: start, end_date: end, icon: null, color: null });
  async function open(off = false) {
    api = createMockApi({ ...fixtures, cycles: [cycle(1, '2026-10-01', '2026-10-14'), cycle(2, '2026-11-16', '2026-11-29')] });
    setApi(api as unknown as Api);
    if (off) await api.settings.set('insights_charts', false);
    const done = (await api.statuses.list()).find((x) => x.category === 'Completed')!;
    for (const cycle_id of [1, 2]) await api.issues.create({ title: `Done ${cycle_id}`, status_id: done.id, cycle_id } as never);
    return put(CyclesPage as never, {});
  }
  it('draws a burn-up with the ideal line for the current cycle', async () => {
    const t = await open();
    await vi.waitFor(() => expect(t.querySelector('[data-group=current] line.ideal')).not.toBeNull());
  });
  it('shows a sparkline on a past cycle row and a velocity chart across cycles', async () => {
    insightsApi.setSource({
      progress: async () => series,
      velocity: async () => [{ start: '2026-10-01', end: '2026-10-14', label: 'Cycle 1', completed: 4 }, { start: '2026-11-16', end: '2026-11-29', label: 'Cycle 2', completed: 2 }],
    });
    const t = await open();
    t.querySelector<HTMLElement>('[data-group=past] [role=button], [data-group=past] button')!.click();
    await wait();
    await vi.waitFor(() => expect(t.querySelector('[data-group=past] svg path.spark')).not.toBeNull());
    expect(t.querySelector('[data-velocity] rect.bar')).not.toBeNull();
  });
  it('takes the velocity bars from the insights api, one cycle scope each', async () => {
    insightsApi.setSource({
      progress: async () => series,
      velocity: async (scope) => ['Cycle' in scope ? scope.Cycle : 0].flatMap((id) => [
        { start: '2026-10-01', end: '2026-10-14', label: 'Cycle 1', completed: id === 1 ? 7 : 99 },
        { start: '2026-11-16', end: '2026-11-29', label: 'Cycle 2', completed: id === 2 ? 3 : 99 },
      ]),
    });
    const t = await open();
    await vi.waitFor(() => expect(t.querySelector('[data-velocity] svg')?.getAttribute('aria-label')).toContain('Cycle 1 7, Cycle 2 3'));
  });
  it('draws no chart when progress charts are off', async () => {
    const t = await open(true);
    expect(t.querySelector('path.scope, path.spark, [data-velocity]')).toBeNull();
  });
});
