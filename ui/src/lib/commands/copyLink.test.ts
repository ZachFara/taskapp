import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, fixtures } from '../api.mock';
import { commandById, listCommands, type Ctx } from './index';

const ctx = (route: string): Ctx => ({ route, layout: null, selection: [], focusedIssueId: null, openIssueId: null, api: createMockApi(fixtures), navigate: () => {} });
let copied: string[];
beforeEach(() => {
  copied = [];
  vi.stubGlobal('navigator', { clipboard: { writeText: async (t: string) => void copied.push(t) } });
});
afterEach(() => vi.unstubAllGlobals());

const run = (id: string, c: Ctx, target?: unknown) => commandById(id)!.run!(c, 1, target);

describe('Copy link commands', () => {
  it('are offered where their item is shown, with a reason for having no key', () => {
    for (const [id, route] of [['project.copyLink', '/project/web'], ['view.copyLink', '/view/1'], ['cycle.copyLink', '/cycles'], ['event.copyLink', '/event/1'], ['event.copyLink', '/events'], ['decision.copyLink', '/decisions'], ['milestone.copyLink', '/project/web']]) {
      const c = commandById(id);
      expect(c, id).toBeDefined();
      expect(listCommands(ctx(route)).some((x) => x.id === id), `${id} on ${route}`).toBe(true);
      expect(c!.noShortcutReason, id).toBeTruthy();
    }
    expect(listCommands(ctx('/decisions')).some((x) => x.id === 'project.copyLink')).toBe(false);
  });

  it('copies the project, view and event named by the page or the row', async () => {
    const c = ctx('/project/x');
    const p = (await c.api.projects.list())[0];
    await run('project.copyLink', ctx(`/project/${p.slug}`));
    await run('project.copyLink', ctx('/projects'), { kind: 'project', id: p.id, name: p.name });
    await run('view.copyLink', ctx('/view/7'));
    await run('view.copyLink', ctx('/views'), { kind: 'view', id: 3, name: 'v' });
    await run('event.copyLink', ctx('/event/5'));
    expect(copied).toEqual([`taskapp://project/${p.slug}`, `taskapp://project/${p.slug}`, 'taskapp://view/7', 'taskapp://view/3', 'taskapp://event/5']);
  });

  it('copies a cycle by its number, and a milestone from a page of the project milestones', async () => {
    const c = ctx('/cycles');
    const cy = (await c.api.cycles.list())[0];
    await run('cycle.copyLink', c, { kind: 'cycle', id: cy.id, name: cy.name });
    expect(copied).toEqual([`taskapp://cycle/${cy.number}`]);

    const ms = fixtures.milestones![0];
    const slug = fixtures.projects.find((p) => p.id === ms.project_id)!.slug;
    const page = commandById('milestone.copyLink')!.page!(ctx(`/project/${slug}`));
    const items = await page.items;
    expect(items.map((i) => i.label)).toContain(ms.name);
    await items.find((i) => i.label === ms.name)!.run();
    expect(copied.at(-1)).toBe(`taskapp://milestone/${ms.id}`);
  });

  it('asks the page to copy on the lists that own their rows', () => {
    for (const [id, route] of [['event.copyLink', '/events'], ['decision.copyLink', '/decisions']]) {
      const seen = vi.fn();
      addEventListener(`command:${id}`, seen);
      void run(id, ctx(route));
      removeEventListener(`command:${id}`, seen);
      expect(seen, id).toHaveBeenCalledOnce();
    }
  });
});
