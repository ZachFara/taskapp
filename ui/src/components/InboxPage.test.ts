import { afterEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import InboxPage from './InboxPage.svelte';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('InboxPage', () => {
  it('inbox_shows_designed_empty_state_only', () => {
    const target = document.createElement('div');
    document.body.append(target);
    live = mount(InboxPage, { target });
    expect(target.textContent).toContain('Inbox arrives in a later release');
    expect(target.querySelector('[data-stub]')).toBeNull();
  });
});
