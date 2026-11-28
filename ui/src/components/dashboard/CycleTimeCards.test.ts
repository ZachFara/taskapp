import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { selectedIssueId } from '../../lib/stores.svelte';
import CycleTimeCard from './CycleTimeCard.svelte';
import LeadTimeCard from './LeadTimeCard.svelte';
import type { Metrics } from './metrics';
import WipCard from './WipCard.svelte';

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) void unmount(app);
  app = null;
  document.body.innerHTML = '';
  selectedIssueId.value = null;
});
const show = (C: typeof CycleTimeCard | typeof LeadTimeCard | typeof WipCard, metrics: Metrics) => {
  app = mount(C, { target: document.body, props: { metrics, measure: 'Count' } });
  flushSync();
};
const m = (over: Partial<Metrics> = {}): Metrics => ({
  from: '2026-11-01', to: '2026-11-07', bucket: 'Day',
  buckets: [
    { start: '2026-11-01', created: 2, completed: 1, completed_points: 1, scope: 5, done_total: 1, wip: 2 },
    { start: '2026-11-02', created: 0, completed: 3, completed_points: 3, scope: 5, done_total: 4, wip: 4 },
  ],
  lead_time: { p50_days: 2, p85_days: 5, n: 6 },
  cycle_time: {
    p50_days: 1, p85_days: 9, n: 3,
    points: [
      { issue_id: 11, key: 'T-11', title: 'First', completed: '2026-11-02', days: 1 },
      { issue_id: 12, key: 'T-12', title: 'Second', completed: '2026-11-05', days: 4 },
      { issue_id: 13, key: 'T-13', title: 'Third', completed: '2026-11-07', days: 9 },
    ],
  },
  wip_now: 3, by_label: [], by_source: [], velocity: [], thin: false, ...over,
});

describe('CycleTimeCard', () => {
  it('draws one dot per finished issue and says how long 85% take', () => {
    show(CycleTimeCard, m());
    expect(document.querySelectorAll('circle[data-issue]')).toHaveLength(3);
    expect(document.body.textContent).toContain('85% finish within 9 days');
  });

  it('opens the issue when its dot is clicked', () => {
    show(CycleTimeCard, m());
    (document.querySelector('circle[data-issue="12"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(selectedIssueId.value).toBe(12);
  });

  it('says so when no issue started and finished in the range', () => {
    show(CycleTimeCard, m({ cycle_time: { p50_days: null, p85_days: null, n: 0, points: [] } }));
    expect(document.querySelectorAll('circle')).toHaveLength(0);
    expect(document.body.textContent).toContain('No finished issue started and finished');
  });
});

describe('LeadTimeCard', () => {
  it('shows the median, the 85th percentile and the count', () => {
    show(LeadTimeCard, m());
    const text = document.body.textContent ?? '';
    expect(text).toContain('2 days');
    expect(text).toContain('5 days');
    expect(text).toContain('6');
  });
});

describe('WipCard', () => {
  it('shows work in progress now and Little\'s Law with the averages', () => {
    show(WipCard, m());
    const text = document.body.textContent ?? '';
    expect(text).toContain('3');
    expect(text).toContain("Little's Law: average cycle time = WIP / throughput");
    // average WIP 3 over 2 days, throughput 2 a day: 1.5 days
    expect(text).toContain('1.5 days');
  });
});
