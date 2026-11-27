import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { CARDS, type CardProps } from './cards';
import type { Metrics } from './metrics';

const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
globalThis.ResizeObserver ??= class { observe() {} unobserve() {} disconnect() {} } as never;

const day = (n: number) => `2026-11-${String(n).padStart(2, '0')}`;
const metrics = (over: Partial<Metrics> = {}): Metrics => ({
  from: day(1), to: day(3), bucket: 'Day',
  buckets: [1, 2, 3].map((n) => ({ start: day(n), created: 2 + n, completed: n, completed_points: n * 2, scope: 4 + n, done_total: n, wip: 1 })),
  lead_time: { p50_days: 2, p85_days: 5, n: 6 }, cycle_time: { p50_days: 1, p85_days: 3, n: 6, points: [] },
  wip_now: 3, by_label: [], by_source: [], velocity: [], thin: false, ...over,
});
const cycle = (n: number) => ({ cycle_id: n, name: `Cycle ${n}`, points: n * 3, count: n });

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) void unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const show = (id: string, props: CardProps) => {
  const C = CARDS.find((c) => c.id === id)!.component;
  app = mount(C, { target: document.body, props });
  flushSync();
};
const values = () => [...document.querySelectorAll('text.val')].map((t) => t.textContent);

describe('Throughput card', () => {
  it('draws completed per bucket as counts, and points when the Points switch is on', () => {
    show('throughput', { metrics: metrics(), measure: 'Count' });
    expect(values()).toEqual(['1', '2', '3']);
    void unmount(app!);
    document.body.innerHTML = '';
    show('throughput', { metrics: metrics(), measure: 'Points' });
    expect(values()).toEqual(['2', '4', '6']);
  });
});

describe('Throughput card with a month of days', () => {
  it('labels only every few days so the dates do not run together', () => {
    const days = Array.from({ length: 30 }, (_, i) => ({ start: `2026-11-${String(i + 1).padStart(2, '0')}`, created: 1, completed: 1, completed_points: 1, scope: 1, done_total: 1, wip: 1 }));
    show('throughput', { metrics: metrics({ buckets: days }), measure: 'Count' });
    const labels = [...document.querySelectorAll('text.tick')].filter((t) => /^\d\d-\d\d$/.test(t.textContent ?? ''));
    expect(labels.length).toBeGreaterThan(3);
    expect(labels.length).toBeLessThan(15);
  });
});

describe('Created vs completed card', () => {
  it('draws two bars per bucket, created and completed, in counts', () => {
    show('created-done', { metrics: metrics(), measure: 'Points' });
    expect(document.querySelectorAll('rect.created')).toHaveLength(3);
    expect(document.querySelectorAll('rect.completed')).toHaveLength(3);
    expect(document.querySelector('svg')!.getAttribute('aria-label')).toContain('3 created, 1 completed');
  });
});

describe('Velocity card', () => {
  it('shows the last five cycles and says the average is over them', () => {
    show('velocity', { metrics: metrics({ velocity: [1, 2, 3, 4, 5, 6].map(cycle) }), measure: 'Count' });
    expect(document.querySelectorAll('rect.bar')).toHaveLength(5);
    expect(document.querySelector('line.avg')).not.toBeNull();
    expect(document.body.textContent).toContain('average of the last 5 cycles');
    expect(document.body.textContent).not.toContain('Cycle 1');
  });
  it('uses points when the Points switch is on, and warns with fewer than three cycles', () => {
    show('velocity', { metrics: metrics({ velocity: [cycle(1), cycle(2)] }), measure: 'Points' });
    expect(values()).toEqual(['3', '6']);
    expect(document.body.textContent).toContain('Fewer than 3 finished cycles');
  });
});

describe('AI vs me card', () => {
  const by_source = [
    { source: 'Ui', created: 3, completed: 2 },
    { source: 'Cli', created: 1, completed: 1 },
    { source: 'Agent', created: 2, completed: 0 },
    { source: 'Migration', created: 4, completed: 0 },
  ];
  it('groups the sources into plain words: you, AI and tools, imported', () => {
    show('ai-vs-me', { metrics: metrics({ by_source }), measure: 'Count' });
    const rows = [...document.querySelectorAll('ul li')].map((li) => li.textContent!.replace(/\s+/g, ' ').trim());
    expect(rows).toEqual(expect.arrayContaining(['You, in the app 3', 'AI and tools 3', 'Imported 4']));
    expect(document.body.textContent).toContain('Created');
    expect(document.body.textContent).toContain('Completed');
  });
});

describe('By label card', () => {
  const by_label = ['a', 'b', 'c', 'd', 'e', 'f', 'g'].map((name, i) => ({ label_id: i, name, completed: 7 - i, points: 2 * (7 - i) }));
  it('shows the top five labels and the rest as Other', () => {
    show('by-label', { metrics: metrics({ by_label }), measure: 'Count' });
    const rows = [...document.querySelectorAll('ul li')].map((li) => li.textContent!.replace(/\s+/g, ' ').trim());
    expect(rows).toEqual(['a 7', 'b 6', 'c 5', 'd 4', 'e 3', 'Other 3']);
  });
  it('sums points when the Points switch is on', () => {
    show('by-label', { metrics: metrics({ by_label: by_label.slice(0, 2) }), measure: 'Points' });
    const rows = [...document.querySelectorAll('ul li')].map((li) => li.textContent!.replace(/\s+/g, ' ').trim());
    expect(rows).toEqual(['a 14', 'b 12']);
  });
});
