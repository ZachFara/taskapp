import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { listCommands, shortcutFor, type Command, type Ctx } from '../../lib/commands';
import BarChart from './BarChart.svelte';
import type { Day } from './geometry';
import ProgressChart from './ProgressChart.svelte';
import Sparkline from './Sparkline.svelte';

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) void unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const show = async (C: typeof ProgressChart | typeof BarChart | typeof Sparkline, props: Record<string, unknown>) => {
  app = mount(C as never, { target: document.body, props });
  await tick();
};
const days: Day[] = [
  { date: '2026-11-01', scope: 10, started: 2, completed: 0 },
  { date: '2026-11-02', scope: 10, started: 4, completed: 2 },
  { date: '2026-11-03', scope: 14, started: 8, completed: 4 },
  { date: '2026-11-04', scope: 14, started: 10, completed: 7 },
  { date: '2026-11-05', scope: 14, started: 12, completed: 9 },
];

describe('ProgressChart', () => {
  it('describes itself as an image with the text summary', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    const svg = document.querySelector('svg[role=img]')!;
    expect(svg.getAttribute('aria-label')).toContain('On the last day: scope 14, started 12, completed 9 (64%).');
  });
  it('draws scope as a stepped line, completed as line plus area, started as a line', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    expect(document.querySelector('path.scope')!.getAttribute('d')).toMatch(/^M32 .*H.*V/);
    expect(document.querySelector('path.completed')).not.toBeNull();
    expect(document.querySelector('path.completed-area')!.getAttribute('d')).toMatch(/Z$/);
    expect(document.querySelector('path.started')).not.toBeNull();
  });
  it('puts the today marker on the day, and draws no actuals after it', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points', today: '2026-11-03' });
    expect(document.querySelector('line.today')!.getAttribute('x1')).toBe('218');
    // completed stops at today: two segments after the first point are gone
    expect(document.querySelector('path.completed')!.getAttribute('d')!.match(/L/g)).toHaveLength(2);
  });
  it('draws the ideal line only when asked', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    expect(document.querySelector('line.ideal')).toBeNull();
    void unmount(app!);
    document.body.innerHTML = '';
    await show(ProgressChart, { days, measure: 'points', unit: 'points', ideal: true });
    expect(document.querySelector('line.ideal')).not.toBeNull();
  });
  it('labels the lines directly, with no legend', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    const text = [...document.querySelectorAll('svg text')].map((t) => t.textContent);
    expect(text).toEqual(expect.arrayContaining(['Scope 14', 'Done 9']));
    expect(document.querySelector('[role=list], .legend')).toBeNull();
  });
  it('draws the stacked variant as bands that close on the baseline', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points', kind: 'stacked' });
    expect(document.querySelectorAll('path.band')).toHaveLength(3);
    expect(document.querySelector('path.scope')).toBeNull();
  });
  it('toggles a data table with one row per day', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    expect(document.querySelector('table')).toBeNull();
    const toggle = [...document.querySelectorAll('button')].find((b) => b.textContent?.includes('View as table'))!;
    toggle.click();
    await tick();
    expect(document.querySelectorAll('table tbody tr')).toHaveLength(5);
    expect(document.querySelector('table')!.textContent).toContain('2026-11-04');
  });
  it('shows a quiet line instead of a chart when nothing has moved', async () => {
    await show(ProgressChart, { days: [], measure: 'points', unit: 'points' });
    expect(document.body.textContent).toContain('Progress appears once issues move');
    expect(document.querySelector('svg[role=img]')).toBeNull();
  });
  it('shows a loading state', async () => {
    await show(ProgressChart, { days: null, measure: 'points', unit: 'points' });
    expect(document.querySelector('[aria-busy=true]')).not.toBeNull();
  });
  it('compact has no table toggle', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points', compact: true });
    expect(document.body.textContent).not.toContain('View as table');
  });
});

describe('BarChart', () => {
  const bars = [{ label: 'Wk 44', value: 8 }, { label: 'Wk 45', value: 12 }, { label: 'Wk 46', value: 10 }];
  it('draws one square bar per period, scaled to the value', async () => {
    await show(BarChart, { bars, unit: 'points', title: 'Completed per week' });
    const rects = [...document.querySelectorAll('rect.bar')];
    expect(rects).toHaveLength(3);
    expect(rects.every((r) => !r.hasAttribute('rx'))).toBe(true);
    const h = rects.map((r) => Number(r.getAttribute('height')));
    expect(h[1]).toBeGreaterThan(h[0]);
    expect(h[1] / h[0]).toBeCloseTo(12 / 8, 1);
  });
  it('labels only the non-zero bars when they are dense', async () => {
    const dense = Array.from({ length: 30 }, (_, i) => ({ label: `d${i}`, value: i === 5 ? 3 : 0 }));
    await show(BarChart, { bars: dense, unit: 'issues', title: 'Completed per day' });
    expect([...document.querySelectorAll('text.val')].map((t) => t.textContent)).toEqual(['3']);
  });
  it('has a text summary and a table toggle', async () => {
    await show(BarChart, { bars, unit: 'points', title: 'Completed per week' });
    expect(document.querySelector('svg[role=img]')!.getAttribute('aria-label')).toBe('Completed per week: Wk 44 8, Wk 45 12, Wk 46 10 points.');
    [...document.querySelectorAll('button')].find((b) => b.textContent?.includes('View as table'))!.click();
    await tick();
    expect(document.querySelectorAll('table tbody tr')).toHaveLength(3);
  });
  it('is quiet when there are no periods', async () => {
    await show(BarChart, { bars: [], unit: 'points', title: 'Completed per week' });
    expect(document.body.textContent).toContain('Progress appears once issues move');
  });
});

describe('Sparkline', () => {
  it('draws a line and an end dot with an accessible name', async () => {
    await show(Sparkline, { values: [0, 2, 5, 9], label: 'Completed over time' });
    expect(document.querySelector('svg')!.getAttribute('aria-label')).toBe('Completed over time');
    expect(document.querySelector('path.spark')!.getAttribute('d')).toMatch(/^M/);
    expect(document.querySelector('circle.end')).not.toBeNull();
  });
  it('draws nothing for fewer than two points', async () => {
    await show(Sparkline, { values: [3], label: 'x' });
    expect(document.querySelector('svg')).toBeNull();
  });
});

describe('ProgressChart keyboard', () => {
  const step = (by: number) => dispatchEvent(new CustomEvent('command:chart.day', { detail: by }));
  const status = () => document.querySelector('[role=status]')!.textContent;
  it('is focusable and moves the highlighted day while it has focus, announcing the values', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    const svg = document.querySelector<SVGElement>('svg[role=img]')!;
    expect(svg.getAttribute('tabindex')).toBe('0');
    step(-1);
    await tick();
    expect(status()).toBe('');
    svg.focus();
    step(-1);
    await tick();
    expect(status()).toBe('11-05: scope 14, started 12, done 9');
    step(-1);
    await tick();
    expect(status()).toBe('11-04: scope 14, started 10, done 7');
    expect(document.querySelector('line.guide')).not.toBeNull();
    for (let i = 0; i < 9; i++) step(-1);
    await tick();
    expect(status()).toContain('11-01');
    step(1);
    await tick();
    expect(status()).toContain('11-02');
  });
  it('has two commands on the arrow keys that are live only while a chart has focus', async () => {
    await show(ProgressChart, { days, measure: 'points', unit: 'points' });
    const all = () => listCommands({} as Ctx, { all: true });
    const prev = all().find((c) => c.id === 'chart.day.prev') as Command;
    expect([shortcutFor('chart.day.prev'), shortcutFor('chart.day.next')]).toEqual(['arrowleft', 'arrowright']);
    expect(prev.when!({} as Ctx)).toBe(false);
    document.querySelector<SVGElement>('svg[role=img]')!.focus();
    expect(prev.when!({} as Ctx)).toBe(true);
  });
});
