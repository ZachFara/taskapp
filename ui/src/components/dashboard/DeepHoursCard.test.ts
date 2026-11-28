import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { Metrics } from './metrics';
import DeepHoursCard from './DeepHoursCard.svelte';

let app: Record<string, unknown> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const text = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';
const show = (deep: Metrics['deep']) => {
  app = mount(DeepHoursCard, { target: document.body, props: { metrics: { bucket: 'Day', deep } as Metrics, measure: 'Count', filter: null, cycle: null, stale: [] } });
  flushSync();
};

describe('DeepHoursCard', () => {
  it('says how to get deep hours when there are none', () => {
    show({ buckets: [], total_hours: 0, streak_days: 0 });
    expect(text()).toContain('No deep focus in this range');
  });

  it('shows the total, the streak and one row per bucket', () => {
    show({ buckets: [{ start: '2026-11-25', hours: 1.5 }, { start: '2026-11-26', hours: 0 }], total_hours: 1.5, streak_days: 2 });
    expect(text()).toContain('1.5 h deep, 2 days in a row');
    expect(document.querySelectorAll('ol.buckets li')).toHaveLength(2);
  });
});
