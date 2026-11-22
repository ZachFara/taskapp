import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import MyIssuesPage from './MyIssuesPage.svelte';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('MyIssuesPage', () => {
  it('my_issues_hides_done_and_canceled_groups', async () => {
    setApi(createMockApi(fixtures));
    const target = document.createElement('div');
    document.body.append(target);
    live = mount(MyIssuesPage, { target });
    await vi.waitFor(() => expect(target.querySelectorAll('.group-head').length).toBeGreaterThan(0));
    flushSync();
    const heads = [...target.querySelectorAll('.group-head')].map((h) => h.textContent?.trim());
    expect(heads.some((h) => h?.startsWith('Done') || h?.startsWith('Canceled'))).toBe(false);
    expect(heads.length).toBeGreaterThan(0);
  });
});
