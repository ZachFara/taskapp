import { afterEach, describe, expect, it } from 'vitest';
import { mount, unmount } from 'svelte';
import { setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import { addDays } from '../lib/gantt';
import TicklerPage from './TicklerPage.svelte';

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

const open = async (api: ReturnType<typeof createMockApi>) => {
  setApi(api);
  app = mount(TicklerPage, { target: document.body });
  await new Promise((r) => setTimeout(r, 10));
};
const today = new Date().toLocaleDateString('sv');

describe('TicklerPage', () => {
  it('lists deferred issues in This week and Later, each with its weekday and day', async () => {
    const api = createMockApi(fixtures);
    const [a, b] = (await api.views.query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false })).slice(0, 2);
    await api.issues.update(a.id, { hidden_until: addDays(today, 3) });
    await api.issues.update(b.id, { hidden_until: addDays(today, 20) });
    await open(api);
    const groups = [...document.querySelectorAll('section.group')].map((g) => [g.getAttribute('aria-label'), [...g.querySelectorAll('li')].map((li) => li.dataset.key)]);
    expect(groups).toEqual([['This week', [a.key]], ['Later', [b.key]]]);
    expect(document.querySelector(`li[data-key="${a.key}"]`)!.textContent).toMatch(/^.*\w{3}, \w{3} \d+/);
  });

  it('shows an empty state when nothing is deferred', async () => {
    await open(createMockApi(fixtures));
    expect(document.body.textContent).toContain('Nothing is deferred');
  });
});
