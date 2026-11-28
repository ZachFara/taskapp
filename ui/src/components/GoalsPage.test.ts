import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import GoalPage from './GoalPage.svelte';
import GoalsPage from './GoalsPage.svelte';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});
beforeEach(() => history.replaceState(null, '', '/goals'));

const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const text = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';
async function seeded() {
  const api = createMockApi(fixtures);
  const p = fixtures.projects[0];
  const g = await api.goals.create('Launch', '2027-Q1', null);
  await api.goals.create('Later', '2027-Q2', null);
  await api.goals.link(g.id, p.id);
  setApi(api);
  return { api, g, p, done: (await api.projects.progress(p.id))[0], total: (await api.projects.progress(p.id))[1] };
}

describe('Goals page', () => {
  it('groups goals by quarter, newest first, with progress from linked projects', async () => {
    const { p, done, total } = await seeded();
    live = mount(GoalsPage, { target: document.body });
    await settle();
    const heads = [...document.querySelectorAll('h2')].map((h) => h.textContent);
    expect(heads).toEqual(['2027-Q2', '2027-Q1']);
    expect(text()).toContain(p.name);
    expect(document.querySelector('[data-goal="1"] [role="img"]')?.getAttribute('aria-label')).toBe(`${done} of ${total} issues done`);
  });

  it('the goal.new command opens the dialog and creating goes to the goal', async () => {
    const { api } = await seeded();
    live = mount(GoalsPage, { target: document.body });
    await settle();
    dispatchEvent(new CustomEvent('command:goal.new'));
    await settle();
    const title = document.querySelector<HTMLInputElement>('input[aria-label="Goal title"]')!;
    title.value = 'Third';
    title.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    [...document.querySelectorAll('button')].find((b) => b.textContent?.includes('Create goal'))!.click();
    await settle();
    expect((await api.goals.list()).map((g) => g.title)).toContain('Third');
    expect(location.pathname).toBe('/goal/3');
  });
});

describe('Goal page', () => {
  it('lists linked projects; achieve, unlink and delete are commands', async () => {
    const { api, g, p } = await seeded();
    history.replaceState(null, '', `/goal/${g.id}`);
    live = mount(GoalPage, { target: document.body, props: { params: { id: String(g.id) } } });
    await settle();
    expect(document.querySelector(`[data-project="${p.id}"]`)).not.toBeNull();
    dispatchEvent(new CustomEvent('command:goal.achieve'));
    await settle();
    expect((await api.goals.list()).find((x) => x.id === g.id)?.status).toBe('achieved');
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Unlink')!.click();
    await settle();
    expect((await api.goals.list()).find((x) => x.id === g.id)?.project_ids).toEqual([]);
    dispatchEvent(new CustomEvent('command:goal.delete'));
    await settle();
    [...document.querySelectorAll('button')].filter((b) => b.textContent?.trim() === 'Delete').at(-1)!.click();
    await settle();
    expect(await api.goals.list()).toHaveLength(1);
    expect(location.pathname).toBe('/goals');
  });
});
