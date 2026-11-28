import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import { features } from '../lib/features.svelte';
import { listCommands, type Ctx } from '../lib/commands';
import ProcessPage from './ProcessPage.svelte';

let live: ReturnType<typeof mount> | null = null;
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
// The owner's statuses on top of the fixtures; the first Backlog-category status plays the Inbox.
const seed = () => ({
  ...fixtures,
  statuses: [
    ...fixtures.statuses,
    ...(['Next', 'Delegated', 'Someday'] as const).map((name, i) => ({ ...fixtures.statuses[0], id: 90 + i, name, position: 90 + i, category: (['Unstarted', 'Started', 'Backlog'] as const)[i] })),
  ],
});
async function open(api: Api) {
  setApi(api);
  live = mount(ProcessPage, { target: document.body });
  await settle();
}
const inbox = (api: Api) => api.inbox.list();
const press = async (name: string) => {
  [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.replace(/\s+/g, ' ').trim().startsWith(name))!.click();
  await settle();
};
const submit = async () => {
  document.querySelector<HTMLButtonElement>('form button[type=submit]')!.click();
  await settle();
};
const progress = () => document.querySelector('.progress')?.textContent;

describe('ProcessPage', () => {
  it('shows the oldest Inbox issue first with its position, and Skip and Back walk the list', async () => {
    const api = createMockApi(seed());
    const all = await inbox(api);
    await open(api);
    expect(progress()).toBe(`1 of ${all.length}`);
    expect(document.querySelector<HTMLInputElement>('input[aria-label=Title]')!.value).toBe(all[0].title);
    await press('Skip');
    expect(progress()).toBe(`2 of ${all.length}`);
    await press('Back');
    expect(progress()).toBe(`1 of ${all.length}`);
  });

  it('Do it now sets Done and moves on, and the issue leaves the Inbox', async () => {
    const api = createMockApi(seed());
    const [first, second] = await inbox(api);
    await open(api);
    await press('Do it now');
    expect((await api.issues.get(first.id)).completed_at).not.toBeNull();
    expect(document.querySelector<HTMLInputElement>('input[aria-label=Title]')!.value).toBe(second.title);
    expect((await inbox(api)).map((i) => i.id)).not.toContain(first.id);
  });

  it('Delegate asks who, then sets Delegated with a Waiting on comment', async () => {
    const api = createMockApi(seed());
    const [first] = await inbox(api);
    await open(api);
    await press('Delegate');
    const field = document.querySelector<HTMLInputElement>('input[aria-label="Waiting on"]')!;
    field.value = 'Sam';
    field.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    await submit();
    expect((await api.comments.list(first.id)).map((c) => c.body)).toEqual(['Waiting on: Sam']);
    const now = await api.issues.get(first.id);
    expect((await api.statuses.list()).find((st) => st.id === now.status_id)!.name).toBe('Delegated');
  });

  it('Defer hides the issue until the chosen day', async () => {
    const api = createMockApi(seed());
    const [first] = await inbox(api);
    await open(api);
    await press('Defer');
    await press('Next week');
    await submit();
    expect((await api.issues.get(first.id)).hidden_until).toMatch(/^\d{4}-\d\d-\d\d$/);
    expect((await inbox(api)).map((i) => i.id)).not.toContain(first.id);
  });

  it('a refused choice shows the error and stays on the issue', async () => {
    // No status called Someday in the plain fixtures.
    const api = createMockApi(fixtures);
    await open(api);
    const before = progress();
    await press('Someday');
    expect(document.querySelector('[role=alert]')!.textContent).toContain('no status called Someday');
    expect(progress()).toBe(before);
  });

  it('ends at Inbox zero once every issue is processed', async () => {
    const api = createMockApi(seed());
    const n = (await inbox(api)).length;
    await open(api);
    for (let i = 0; i < n; i++) await press('Trash');
    expect(document.body.textContent).toContain('Inbox zero');
    expect(document.body.textContent).toContain(`You processed ${n}`);
  });

  it('skipped issues are offered again at the end instead of Inbox zero', async () => {
    const api = createMockApi(seed());
    const n = (await inbox(api)).length;
    await open(api);
    await press('Trash');
    for (let i = 1; i < n; i++) await press('Skip');
    expect(document.body.textContent).toContain('End of the list');
    await press('Go through them again');
    expect(progress()).toBe(`1 of ${n - 1}`);
  });
});

describe('the process_inbox switch', () => {
  const ctx = (route: string) => ({ route, selection: [], focusedIssueId: null, openIssueId: null, api: {} as Ctx['api'], navigate: () => {} }) as Ctx;
  const live = (route: string) => listCommands(ctx(route)).map((c) => c.id).filter((id) => id === 'nav.process' || id.startsWith('process.'));
  afterEach(() => (features.process_inbox = true));

  it('hides the commands when off and shows them when on', () => {
    expect(live('/')).toEqual(['nav.process']);
    expect(live('/process')).toContain('process.do');
    features.process_inbox = false;
    expect(live('/')).toEqual([]);
    expect(live('/process')).toEqual([]);
  });
});
