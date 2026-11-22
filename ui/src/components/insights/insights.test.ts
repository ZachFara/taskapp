import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';
import { clickLabel, pick } from '../settings/testing';
import { loadConfig, saveConfig } from './config';
import { measuresFor, pointsOf, readEstimation, SCALES } from './estimation';
import { insightsApi, type ProgressSeries } from './insightsApi';
import InsightsPanel from './InsightsPanel.svelte';

const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
proto.scrollIntoView ??= () => {};

const flush = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const series: ProgressSeries = {
  measure: 'points', from: '2026-11-01', to: '2026-11-05',
  days: [
    { date: '2026-11-01', scope: 10, started: 2, completed: 0 },
    { date: '2026-11-02', scope: 10, started: 4, completed: 2 },
    { date: '2026-11-03', scope: 14, started: 8, completed: 4 },
    { date: '2026-11-04', scope: 14, started: 10, completed: 7 },
    { date: '2026-11-05', scope: 20, started: 12, completed: 10 },
  ],
};
let api: ReturnType<typeof createMockApi>;
let app: ReturnType<typeof mount> | null = null;
const asked: unknown[][] = [];
beforeEach(() => {
  api = createMockApi();
  setApi(api as unknown as Api);
  asked.length = 0;
  insightsApi.setSource({
    progress: async (...a) => (asked.push(a), series),
    velocity: async () => [{ start: '2026-11-02', end: '2026-11-08', label: 'Wk 45', completed: 6 }, { start: '2026-11-09', end: '2026-11-15', label: 'Wk 46', completed: 9 }],
  });
});
afterEach(() => {
  if (app) void unmount(app);
  app = null;
  document.body.innerHTML = '';
  insightsApi.setSource(null);
});
const open = async (id = 1, kind: 'view' | 'project' | 'cycle' = 'view') => {
  if (app) void unmount(app);
  document.body.innerHTML = '';
  app = mount(InsightsPanel, { target: document.body, props: { kind, id, title: 'Sprint view', onclose: () => {} } });
  await flush();
};
const optionTexts = () => [...document.querySelectorAll('[role=option]')].map((o) => o.textContent?.trim());
const choose = (label: string, option: string) => pick(document.body, label, option);

describe('estimation settings', () => {
  it('measures follow the setting', () => {
    expect(measuresFor('none')).toEqual(['count']);
    expect(measuresFor('time')).toEqual(['count', 'time']);
    expect(measuresFor('points')).toEqual(['points', 'count']);
    expect(measuresFor('both')).toEqual(['points', 'count', 'time']);
  });
  it('defaults to both and fibonacci, and ignores junk', async () => {
    expect(await readEstimation()).toEqual({ estimation: 'both', scale: 'fibonacci' });
    await api.settings.set('estimation', 'banana');
    await api.settings.set('point_scale', 'tshirt');
    expect(await readEstimation()).toEqual({ estimation: 'both', scale: 'tshirt' });
  });
  it('scales hold the agreed values', () => {
    expect(SCALES.fibonacci.map((s) => s.value)).toEqual([0, 1, 2, 3, 5, 8, 13, 21]);
    expect(SCALES.linear.map((s) => s.value)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(SCALES.powers.map((s) => s.value)).toEqual([1, 2, 4, 8, 16]);
    expect(SCALES.tshirt.map((s) => [s.label, s.value])).toEqual([['XS', 1], ['S', 2], ['M', 3], ['L', 5], ['XL', 8]]);
  });
  it('reads points defensively', () => {
    expect(pointsOf({} as never)).toBeNull();
    expect(pointsOf({ points: 3 } as never)).toBe(3);
    expect(pointsOf({ points: 'x' } as never)).toBeNull();
  });
});

describe('config', () => {
  it('is remembered per entity under insights.<kind>.<id>', async () => {
    await saveConfig('view', 1, { measure: 'count', chart: 'stacked', range: '90' });
    expect(await api.settings.get('insights.view.1')).toEqual(expect.any(String));
    expect(await loadConfig('view', 1, ['points', 'count'])).toEqual({ measure: 'count', chart: 'stacked', range: '90' });
    expect(await loadConfig('view', 2, ['points', 'count'])).toEqual({ measure: 'points', chart: 'burnup', range: 'auto' });
  });
  it('falls back when the stored value is junk or names a measure that is not allowed', async () => {
    await api.settings.set('insights.view.3', '{nope');
    expect((await loadConfig('view', 3, ['count'])).measure).toBe('count');
    await saveConfig('view', 4, { measure: 'time', chart: 'burnup', range: 'auto' });
    expect((await loadConfig('view', 4, ['count'])).measure).toBe('count');
  });
});

describe('InsightsPanel', () => {
  it('shows scope, started, completed and percent from the last day, and the chart', async () => {
    await open();
    const head = document.querySelector('[data-headline]')!.textContent!.replace(/\s+/g, ' ');
    expect(head).toContain('20 Scope');
    expect(head).toContain('12 Started');
    expect(head).toContain('10 Completed');
    expect(head).toContain('50% Done');
    expect(document.querySelector('svg[role=img]')).not.toBeNull();
  });
  it('asks for the entity as the scope and the chosen measure', async () => {
    await open(7, 'cycle');
    expect(asked[0].slice(0, 2)).toEqual([{ Cycle: 7 }, 'points']);
  });
  it('offers only the measures the estimation setting allows', async () => {
    await api.settings.set('estimation', 'time');
    await open();
    await clickLabel('Measure');
    expect(optionTexts()).toEqual(['Issues', 'Time']);
  });
  it('offers points, issues and time when estimation is both', async () => {
    await open();
    await clickLabel('Measure');
    expect(optionTexts()).toEqual(['Points', 'Issues', 'Time']);
  });
  it('remembers the chart choice per view', async () => {
    await open(1);
    expect(document.querySelector('path.scope')).not.toBeNull();
    await choose('Chart', 'Stacked area');
    expect(document.querySelector('path.band')).not.toBeNull();
    await open(1);
    expect(document.querySelector('path.band')).not.toBeNull();
    await open(2);
    expect(document.querySelector('path.scope')).not.toBeNull();
  });
  it('re-asks the backend when the measure changes', async () => {
    await open();
    await choose('Measure', 'Issues');
    expect(asked.at(-1)![1]).toBe('count');
  });
  it('shows a velocity bar chart when there is history', async () => {
    await open();
    expect(document.querySelectorAll('rect.bar')).toHaveLength(2);
  });
  it('is quiet when the backend has nothing yet', async () => {
    insightsApi.setSource({ progress: async () => null, velocity: async () => [] });
    await open();
    expect(document.body.textContent).toContain('Progress appears once issues move');
  });
});
