import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';
import { listCommands, type Ctx } from '../../lib/commands';
import Sidebar from '../Sidebar.svelte';
import { CARDS } from './cards';
import DashboardPage from './DashboardPage.svelte';
import { dashboard, moveCard, orderedCards, toggleCard } from './dashboard.svelte';
import { features, setFeature } from '../../lib/features.svelte';
import type { Metrics, MetricsQuery } from './metrics';

const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
proto.scrollIntoView ??= () => {};
globalThis.ResizeObserver ??= class { observe() {} unobserve() {} disconnect() {} } as never;

const settle = async () => {
  for (let i = 0; i < 12; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const day = (n: number) => `2026-11-${String(n).padStart(2, '0')}`;
const metrics = (over: Partial<Metrics> = {}): Metrics => ({
  from: day(1), to: day(3), bucket: 'Day',
  buckets: [1, 2, 3].map((n) => ({ start: day(n), created: 2, completed: n, completed_points: n * 2, scope: 4 + n, done_total: n, wip: 1 })),
  lead_time: { p50_days: 2, p85_days: 5.5, n: 6 }, cycle_time: { p50_days: 1, p85_days: 3, n: 6, points: [] },
  wip_now: 3, by_label: [], by_source: [], velocity: [], thin: false, ...over,
});

let app: ReturnType<typeof mount> | null = null;
const asked: MetricsQuery[] = [];
async function open(answer: (q: MetricsQuery) => Promise<Metrics> | Metrics, component: typeof DashboardPage | typeof Sidebar = DashboardPage) {
  const api = createMockApi() as unknown as Api & { metrics: { for: (q: MetricsQuery) => Promise<Metrics> } };
  api.metrics.for = async (q) => (asked.push(q), answer(q));
  setApi(api);
  app = mount(component as typeof DashboardPage, { target: document.body });
  await settle();
}
beforeEach(() => {
  localStorage.clear();
  features.dashboard = true;
  Object.assign(dashboard, { range: 'month', from: '', to: '', measure: 'Count', order: [], hidden: [], project: null, label: null });
  asked.length = 0;
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const text = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';

describe('DashboardPage', () => {
  it('asks the engine for the range with a day bucket and shows the summary and burn-up cards', async () => {
    await open(() => metrics());
    expect(asked[0]).toMatchObject({ bucket: 'Day', measure: 'count', filter: null });
    expect('Range' in asked[0].scope).toBe(true);
    const s = document.querySelector('section[aria-label="Summary"]')!;
    expect(s.textContent).toContain('Completed');
    expect(s.textContent).toContain('5.5 d');
    expect(document.querySelector('section[aria-label="Burn-up"]')).not.toBeNull();
    expect(text()).not.toContain('Fewer than 5 issues finished');
  });

  it('says so when the data is thin', async () => {
    await open(() => metrics({ thin: true, lead_time: { p50_days: null, p85_days: null, n: 0 } }));
    expect(text()).toContain('Fewer than 5 issues finished');
    expect(text()).toContain('–');
  });

  it('shows a skeleton while loading, an empty state, and an error', async () => {
    let release: (m: Metrics) => void = () => {};
    await open(() => new Promise<Metrics>((r) => (release = r)));
    expect(document.querySelector('[aria-busy="true"]')).not.toBeNull();
    release(metrics({ buckets: [] }));
    await settle();
    expect(text()).toContain('Nothing in this range');
    unmount(app!);
    document.body.innerHTML = '';
    await open(() => Promise.reject({ message: 'engine down' }));
    expect(document.querySelector('[role="alert"]')?.textContent).toBe('engine down');
  });

  it('hidden cards stay hidden and the order is kept', async () => {
    const ids = CARDS.map((c) => c.id);
    ids.slice(1).forEach(toggleCard);
    await open(() => metrics());
    expect(document.querySelector('section[aria-label="Burn-up"]')).toBeNull();
    for (const c of CARDS) if (!dashboard.hidden.includes(c.id)) toggleCard(c.id);
    await settle();
    expect(text()).toContain('All cards are hidden');
    moveCard('summary', 1);
    expect(orderedCards().slice(0, 2)).toEqual(['burnup', 'summary']);
    const saved = JSON.parse(localStorage.getItem('cairn.dashboard')!);
    expect(saved.order.slice(0, 2)).toEqual(['burnup', 'summary']);
    expect([...saved.hidden].sort()).toEqual(CARDS.map((c) => c.id).sort());
  });

  it('applies the saved project and label filter on load', async () => {
    localStorage.setItem('cairn.dashboard.project', '7');
    localStorage.setItem('cairn.dashboard.label', '3');
    vi.resetModules();
    const fresh = await import('./dashboard.svelte');
    expect([fresh.dashboard.project, fresh.dashboard.label]).toEqual([7, 3]);
    fresh.set({ project: null });
    expect(localStorage.getItem('cairn.dashboard.project')).toBeNull();
    localStorage.removeItem('cairn.dashboard.label');
  });

  it('a cycle chosen in Range links to its retrospective', async () => {
    await open(() => metrics());
    expect([...document.querySelectorAll('button')].some((b) => b.textContent?.trim() === 'Retrospective')).toBe(false);
    const id = (await createMockApi().cycles.list())[0].id;
    dashboard.range = `cycle:${id}`;
    await settle();
    const link = [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Retrospective')!;
    link.click();
    expect(location.pathname + location.hash).toMatch(/^\/cycles#\d+$/);
    history.replaceState(null, '', '/');
  });

  it('the Points switch asks again with Points', async () => {
    await open(() => metrics());
    dashboard.measure = 'Points';
    await settle();
    expect(asked.at(-1)!.measure).toBe('points');
    expect(text()).toContain('Points completed');
  });
});

describe('the Dashboard switch', () => {
  const ctx = (route: string) => ({ route, api: createMockApi(), selection: [], focusedIssueId: null, openIssueId: null, navigate: () => {} }) as unknown as Ctx;
  const ids = (route: string) => listCommands(ctx(route)).map((c) => c.id).filter((id) => id.includes('dashboard') && !id.startsWith('feature.'));

  it('turns the sidebar entry and the commands off and on', async () => {
    await open(() => metrics(), Sidebar);
    expect(document.querySelector('a[href="/dashboard"]')).not.toBeNull();
    expect(ids('/dashboard')).toEqual(expect.arrayContaining(['nav.dashboard', 'dashboard.range', 'dashboard.measure', 'dashboard.filter', 'dashboard.cards', 'dashboard.cards.move']));
    expect(ids('/my')).toEqual(['nav.dashboard']);
    await setFeature('dashboard', false);
    await settle();
    expect(document.querySelector('a[href="/dashboard"]')).toBeNull();
    expect(ids('/dashboard')).toEqual([]);
  });

  it('is read from the setting, default on', async () => {
    const api = createMockApi();
    await api.settings.set('dashboard', false);
    setApi(api);
    app = mount(Sidebar, { target: document.body });
    await settle();
    expect(document.querySelector('a[href="/dashboard"]')).toBeNull();
  });
});
