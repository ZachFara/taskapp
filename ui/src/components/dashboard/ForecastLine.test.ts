import { afterEach, describe, expect, it } from 'vitest';
import { mount, unmount } from 'svelte';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';
import type { Forecast, ForecastQuery } from '../../lib/bindings';
import ForecastLine from './ForecastLine.svelte';

const base: Forecast = { open: 4, completions: 20, window_days: 56, enough: true, p50: '2030-01-12', p85: '2030-01-20' };
let m: ReturnType<typeof mount> | undefined;
afterEach(() => (m && unmount(m), (m = undefined), (document.body.innerHTML = '')));

async function show(answer: Partial<Forecast>, props: { filter?: null; cycle?: number | null } = {}) {
  const api = createMockApi() as unknown as Api;
  const asked: ForecastQuery[] = [];
  api.forecast.for = async (q) => (asked.push(q), { ...base, ...answer });
  setApi(api);
  m = mount(ForecastLine, { target: document.body, props });
  await expect.poll(() => document.body.textContent).not.toContain('Forecasting');
  return asked;
}

describe('ForecastLine', () => {
  it('says_likely_and_85_percent_dates', async () => {
    await show({});
    expect(document.body.textContent).toContain('At the current pace: likely by Jan 12, 2030, 85% by Jan 20, 2030.');
  });
  it('says_not_enough_history', async () => {
    await show({ enough: false, p50: null, p85: null, completions: 3 });
    expect(document.body.textContent).toContain('Not enough history to forecast: 3 finished in the last 56 days');
  });
  it('says_nothing_is_open', async () => {
    await show({ open: 0, p50: null, p85: null });
    expect(document.body.textContent).toContain('Nothing is open.');
  });
  it('asks_for_its_cycle', async () => {
    const asked = await show({}, { cycle: 7 });
    expect(asked).toEqual([{ filter: null, cycle: 7 }]);
  });
});
