import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import ReviewPage from './ReviewPage.svelte';
import { start } from './review/review.svelte';

let live: ReturnType<typeof mount> | null = null;
beforeEach(() => start());
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
async function open(api: Api) {
  setApi(api);
  live = mount(ReviewPage, { target: document.body });
  await settle();
}
const button = (name: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.replace(/\s+/g, ' ').trim() === name)!;
const press = async (name: string) => {
  button(name).click();
  await settle();
};

describe('ReviewPage', () => {
  it('walks the steps one at a time, starting at Inbox to zero with its count', async () => {
    const api = createMockApi(fixtures);
    const inbox = await api.inbox.list();
    await open(api);
    expect(document.querySelector('h2')?.textContent).toBe('Inbox to zero');
    expect(document.querySelector('.count')?.textContent).toBe(`${inbox.length} in Inbox`);
    await press('Next');
    expect(document.querySelector('h2')?.textContent).toBe('Waiting');
    await press('Back');
    expect(document.querySelector('h2')?.textContent).toBe('Inbox to zero');
  });

  it('Finish stays off until every step is ticked, then records last_review as a day', async () => {
    const api = createMockApi(fixtures);
    const set = vi.spyOn(api.settings, 'set');
    await open(api);
    expect(button('Finish').disabled).toBe(true);
    for (let i = 0; i < 6; i++) {
      await press('Done for this step');
      if (i < 5) await press('Next');
    }
    expect(button('Finish').disabled).toBe(false);
    await press('Finish');
    expect(set).toHaveBeenCalledWith('last_review', expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
    expect(await api.settings.get('last_review')).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});
