import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../lib/api.mock';
import { setApi, type Api } from '../lib/api';
import FocusPage from './FocusPage.svelte';

let app: Record<string, unknown> | null = null;
let api: Api;
beforeEach(() => {
  api = createMockApi();
  setApi(api);
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const text = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';
const tick = () => new Promise((r) => setTimeout(r, 20));

describe('FocusPage', () => {
  it('shows the issue, starts a session, and pauses it', async () => {
    const issue = await api.issues.create({ title: 'Write the spec' });
    app = mount(FocusPage, { target: document.body, props: { params: { key: issue.key } } });
    await tick();
    flushSync();
    expect(text()).toContain(issue.title);
    expect(text()).toContain('Start');
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Start')?.click();
    await tick();
    flushSync();
    expect(await api.focus.running()).toMatchObject({ issue_id: issue.id, ended_at: null });
    expect(text()).toContain('Pause');
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Pause')?.click();
    await tick();
    flushSync();
    expect(await api.focus.running()).toBeNull();
    expect(text()).toContain('Resume');
  });

  it('says so for an unknown key', async () => {
    app = mount(FocusPage, { target: document.body, props: { params: { key: 'ABC-99999' } } });
    await tick();
    flushSync();
    expect(text()).toContain('No such issue');
  });
});
