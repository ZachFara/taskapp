import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { listCommands, type Ctx } from '../lib/commands';
import NextPage from './NextPage.svelte';
import StuckCard from './dashboard/StuckCard.svelte';

const settle = async () => {
  for (let i = 0; i < 30; i++) await Promise.resolve();
  await tick();
};
let app: ReturnType<typeof mount> | undefined;
afterEach(() => app && unmount(app));

describe('the Next actions page', () => {
  it('lists the next action of a project and marks an active project without one as stuck', async () => {
    const api = createMockApi();
    setApi(api);
    const [a, b] = [await api.projects.create('alpha', 'Alpha', null, null), await api.projects.create('beta', 'Beta', null, null)];
    for (const p of [a, b]) await api.projects.update({ ...p, status_id: 2 });
    const i = await api.issues.create({ title: 'Call the bank', project_id: a.id } as never);
    await api.projects.setNext(a.id, i.id);
    app = mount(NextPage, { target: document.body });
    await settle();
    const rows = [...document.querySelectorAll('tbody tr')];
    expect(rows.find((r) => r.getAttribute('data-slug') === 'alpha')?.textContent).toContain('Call the bank');
    expect(rows.find((r) => r.getAttribute('data-slug') === 'beta')?.textContent).toContain('Stuck');
  });
});

describe('the Stuck projects card', () => {
  it('names each stuck project', async () => {
    const api = createMockApi();
    const p = await api.projects.create('gamma', 'Gamma', null, null);
    await api.projects.update({ ...p, status_id: 2 });
    const stuck = (await api.next.list()).filter((n) => n.stuck);
    app = mount(StuckCard, { target: document.body, props: { stuck } as never });
    await settle();
    expect(document.body.textContent).toContain('Gamma');
  });
});

describe('Make next action', () => {
  it('flags the focused issue for its project', async () => {
    const api = createMockApi();
    setApi(api);
    const p = await api.projects.create('delta', 'Delta', null, null);
    const i = await api.issues.create({ title: 'Do it', project_id: p.id } as never);
    const ctx = { route: '/view/1', layout: 'list', selection: [], focusedIssueId: i.id, openIssueId: null, api, navigate: () => {} } as Ctx;
    await listCommands(ctx).find((c) => c.id === 'issue.next')!.run!(ctx);
    expect((await api.next.list()).find((n) => n.project.id === p.id)?.issue?.id).toBe(i.id);
  });
});
