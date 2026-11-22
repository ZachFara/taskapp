import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import ViewsPage from './ViewsPage.svelte';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

async function open(mock = createMockApi(fixtures)) {
  setApi(mock);
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(ViewsPage, { target });
  await vi.waitFor(() => expect(target.querySelectorAll('[data-view-id]').length).toBeGreaterThan(0));
  flushSync();
  return { mock, target };
}

describe('ViewsPage', () => {
  it('views_show_layout_and_pinned_state', async () => {
    const { target } = await open();
    const first = target.querySelector<HTMLElement>(`[data-view-id="${fixtures.views![0].id}"]`)!;
    expect(first.textContent).toContain(fixtures.views![0].name);
    expect(first.textContent).toContain(fixtures.views![0].def.layout);
  });

  it('delete_view_asks_before_removing', async () => {
    const { mock, target } = await open();
    const del = vi.spyOn(mock.views, 'delete');
    const id = fixtures.views![0].id;
    target.querySelector<HTMLElement>(`[data-view-id="${id}"]`)!.querySelector<HTMLButtonElement>('button[aria-label^="Delete"]')!.click();
    flushSync();
    expect(del).not.toHaveBeenCalled();
    [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === 'Delete view')!.click();
    await vi.waitFor(() => expect(del).toHaveBeenCalledWith(id));
  });
});
