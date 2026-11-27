import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setApi } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import type { Measure, ProgressScope } from '../../lib/bindings';
import ProgressChart from './ProgressChart.svelte';

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) void unmount(app);
  app = null;
  document.body.innerHTML = '';
});

// The accessible table must carry exactly the numbers the backend series has, for the seeded project and cycle.
describe('View as table equals the backend series', () => {
  const scopes: [string, ProgressScope][] = [['project', { Project: fixtures.projects.find((p) => p.start_date && p.target_date)!.id }], ['cycle', { Cycle: fixtures.cycles![0].id }]];
  for (const [name, scope] of scopes)
    for (const measure of ['issues', 'points'] as Measure[])
      it(`${name} / ${measure}`, async () => {
        const api = createMockApi();
        setApi(api);
        const p = await api.insights.progress(scope, measure, null, null);
        expect(p.days.length).toBeGreaterThan(1);
        const days = p.days.map((d) => ({ date: d.date, scope: d.scope ?? 0, started: d.started ?? 0, completed: d.completed ?? 0 }));
        app = mount(ProgressChart, { target: document.body, props: { days, measure, unit: measure } });
        await tick();
        [...document.querySelectorAll('button')].find((b) => b.textContent?.includes('View as table'))!.click();
        await tick();
        const rows = [...document.querySelectorAll('table tbody tr')].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent));
        const round = (n: number) => String(Math.round(n * 10) / 10);
        expect(rows).toEqual(days.map((d) => [d.date, round(d.scope), round(d.started), round(d.completed)]));
      });
});
