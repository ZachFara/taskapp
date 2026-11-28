import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../lib/api.mock';
import { setApi, type Api } from '../lib/api';
import { features } from '../lib/features.svelte';
import { openShutdown, shutdown } from '../lib/shutdown.svelte';
import ShutdownPanel from './ShutdownPanel.svelte';

let app: Record<string, unknown> | null = null;
let api: Api;
beforeEach(() => {
  api = createMockApi();
  setApi(api);
  features.shutdown = true;
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  shutdown.open = false;
  document.body.innerHTML = '';
});
const text = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';
const tick = () => new Promise((r) => setTimeout(r, 20));
const button = (label: string) => [...document.querySelectorAll('button')].find((b) => b.textContent?.trim().toLowerCase().includes(label));

describe('ShutdownPanel', () => {
  it('shows the Inbox count, the deep issues to choose from and the streak', async () => {
    const a = await api.issues.create({ title: 'Write the spec' });
    await api.issues.update(a.id, { depth: 'deep' });
    app = mount(ShutdownPanel, { target: document.body });
    await openShutdown();
    await tick();
    flushSync();
    expect(text()).toContain('Inbox reviewed');
    expect(text()).toContain('Write the spec');
    expect(text()).toContain('Finish today to start a streak');
  });

  it('saves tomorrow\'s picks, captures a loose end and finishes the day', async () => {
    const a = await api.issues.create({ title: 'Write the spec' });
    await api.issues.update(a.id, { depth: 'deep' });
    app = mount(ShutdownPanel, { target: document.body });
    await openShutdown();
    await tick();
    flushSync();

    document.querySelector<HTMLElement>('[role="checkbox"][aria-label="Write the spec"]')?.click();
    await tick();
    button("save tomorrow")?.click();
    await tick();
    expect((await api.shutdown.state()).tomorrow.map((i) => i.title)).toEqual(['Write the spec']);

    shutdown.loose = 'Call the bank';
    flushSync();
    button('capture')?.click();
    await tick();
    expect(shutdown.loose).toBe('');
    expect(JSON.stringify(await api.inbox.list())).toContain('Call the bank');

    button('finish')?.click();
    await tick();
    flushSync();
    expect(text()).toContain('Shutdown complete');
    expect((await api.shutdown.state()).done_today).toBe(true);
  });
});
