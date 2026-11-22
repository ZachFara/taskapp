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
    const mock = createMockApi(fixtures);
    setApi(mock);
    const sts = await mock.statuses.list();
    const open = sts.find((x) => x.category === 'Started')!;
    const done = sts.find((x) => x.category === 'Completed')!;
    const cancelled = sts.find((x) => x.category === 'Canceled')!;
    for (const st of [open, done, cancelled]) await mock.issues.create({ title: `Mine ${st.name}`, status_id: st.id } as never);
    const target = document.createElement('div');
    document.body.append(target);
    live = mount(MyIssuesPage, { target });
    await vi.waitFor(() => expect(target.querySelectorAll('.band .name').length).toBeGreaterThan(0));
    flushSync();
    const heads = [...target.querySelectorAll('.band .name')].map((h) => h.textContent?.trim());
    expect(heads).toContain(open.name);
    expect(heads).not.toContain(done.name);
    expect(heads).not.toContain(cancelled.name);
  });
});
