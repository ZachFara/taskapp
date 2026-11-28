import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { api, setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import { setLinkPrompt } from '../lib/linkPrompt.svelte';
import IssueLinks from './detail/IssueLinks.svelte';
import LinkSolution from './LinkSolution.svelte';

vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(async () => {}) }));

let live: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  localStorage.clear();
  setLinkPrompt(true);
  setApi(createMockApi(structuredClone(fixtures)));
});
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
const input = (label: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
const type = (el: HTMLInputElement, v: string) => {
  el.value = v;
  el.dispatchEvent(new Event('input', { bubbles: true }));
};
const submit = (el: Element) => el.closest('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

async function detail() {
  const issue = (await api.views.query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }))[0];
  // The page reloads on its own `rev`; a write here just mounts the section again.
  const act = async (p: Promise<unknown>) => {
    await p.catch(() => {});
    unmount(live!);
    live = mount(IssueLinks, { target: document.body, props: { issue, act, rev: 0 } });
  };
  live = mount(IssueLinks, { target: document.body, props: { issue, act, rev: 0 } });
  await settle();
  return issue;
}

describe('IssueLinks', () => {
  it('adds a link with a title and kind, lists it, and removes it', async () => {
    const issue = await detail();
    expect(document.body.textContent).toContain('No links');
    type(input('Link URL'), 'https://example.com/pr/1');
    type(input('Link title'), 'The fix');
    submit(input('Link URL'));
    await settle();
    expect(await api.links.list(issue.id)).toMatchObject([{ url: 'https://example.com/pr/1', title: 'The fix', kind: 'reference' }]);
    expect(document.querySelector('.go')!.textContent).toContain('The fix');
    document.querySelector<HTMLButtonElement>('button[aria-label^="Remove link"]')!.click();
    await settle();
    expect(await api.links.list(issue.id)).toEqual([]);
  });

  it('shows the refusal for a URL the backend rejects', async () => {
    await detail();
    type(input('Link URL'), 'javascript:alert(1)');
    submit(input('Link URL'));
    await settle();
    expect(document.querySelectorAll('.go')).toHaveLength(0);
  });

  it('opens a web link through the link guard and refuses what the guard refuses', async () => {
    const issue = await detail();
    const opener = vi.spyOn(window, 'open').mockReturnValue(null);
    await api.links.add(issue.id, 'https://example.com/a', null, 'pr');
    await api.links.add(issue.id, 'obsidian://new?vault=v&content=x', null, 'doc');
    await detail();
    const rows = document.querySelectorAll<HTMLButtonElement>('.go');
    rows[0].click();
    await settle();
    expect(opener).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener');
    rows[1].click();
    await settle();
    expect(opener).toHaveBeenCalledTimes(1);
    expect(document.querySelector('[role="alert"]')!.textContent).toContain('does not open');
  });

  it('opens a taskapp link inside the app', async () => {
    const issue = await detail();
    await api.links.add(issue.id, 'taskapp://event/9', 'Offsite', 'reference');
    unmount(live!);
    await detail();
    document.querySelector<HTMLButtonElement>('.go')!.click();
    await settle();
    expect(location.pathname).toBe('/event/9');
  });
});

describe('LinkSolution', () => {
  const completed = (a: Api) => a.statuses.list().then((l) => l.find((s) => s.category === 'Completed')!);
  async function move(statusOf: (a: Api) => Promise<{ id: number }>, ids?: number[]) {
    const issues = await api.views.query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
    const target = ids ?? [issues[0].id];
    const st = await statusOf(api);
    if (target.length === 1) await api.issues.update(target[0], { status_id: st.id } as never);
    else await api.issues.bulkUpdate(target, { patch: { status_id: st.id } } as never);
    await settle();
    return target[0];
  }
  const start = async () => {
    live = mount(LinkSolution, { target: document.body });
    await settle();
  };

  it('asks after one issue is moved to a Completed status, and a pasted URL becomes a solution link', async () => {
    await start();
    const id = await move(completed);
    expect(document.querySelector('[aria-label="Link the solution"]')!.textContent).toContain('Link the solution to');
    type(input('Solution URL'), 'https://example.com/pr/7');
    submit(input('Solution URL'));
    await settle();
    expect(await api.links.list(id)).toMatchObject([{ url: 'https://example.com/pr/7', kind: 'solution' }]);
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
  });

  it('stays quiet for a status that is not Completed, a bulk move, and an issue that already has a solution', async () => {
    await start();
    const started = (a: Api) => a.statuses.list().then((l) => l.find((s) => s.category === 'Started')!);
    await move(started);
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
    const issues = await api.views.query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
    await move(completed, [issues[1].id, issues[2].id]);
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
    await api.links.add(issues[3].id, 'https://example.com/done', null, 'solution');
    await move(completed, [issues[3].id]);
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
  });

  it('Skip closes it without linking, and Do not ask again turns it off for later moves', async () => {
    await start();
    const id = await move(completed);
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Skip')!.click();
    flushSync();
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
    expect(await api.links.list(id)).toEqual([]);

    const issues = await api.views.query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
    await move(completed, [issues[1].id]);
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === "Don't ask again")!.click();
    flushSync();
    await move(completed, [issues[2].id]);
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
  });

  it('Escape inside it closes it', async () => {
    await start();
    await move(completed);
    document.querySelector('[aria-label="Link the solution"]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    flushSync();
    expect(document.querySelector('[aria-label="Link the solution"]')).toBeNull();
  });
});
