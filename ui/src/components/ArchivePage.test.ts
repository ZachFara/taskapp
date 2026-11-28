import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { listCommands, type Ctx } from '../lib/commands';
import { archivedDef, deleteAsk } from '../lib/deleteIssues.svelte';
import { selection } from '../lib/selection.svelte';
import ArchivePage from './ArchivePage.svelte';
import DeleteIssues from './DeleteIssues.svelte';

const settle = async () => {
  for (let i = 0; i < 40; i++) await Promise.resolve();
  await tick();
};
let apps: ReturnType<typeof mount>[] = [];
afterEach(() => {
  apps.forEach((a) => unmount(a));
  apps = [];
  deleteAsk.n = 0;
  document.body.innerHTML = '';
});
const ctx = (api: ReturnType<typeof createMockApi>, route: string, ids: number[]): Ctx => ({ route, selection: ids, focusedIssueId: null, openIssueId: null, api, navigate: () => {} });
const run = (api: ReturnType<typeof createMockApi>, id: string, route: string, ids: number[]) => {
  const c = ctx(api, route, ids);
  const cmd = listCommands(c, { all: true }).find((x) => x.id === id)!;
  expect(!cmd.when || cmd.when(c), `${id} listed on ${route}`).toBe(true);
  return cmd.run!(c);
};

async function setup() {
  const api = createMockApi();
  setApi(api);
  const [a, b] = [await api.issues.create({ title: 'Old thing' } as never), await api.issues.create({ title: 'Older thing' } as never)];
  await api.issues.archive(a.id);
  await api.issues.archive(b.id);
  apps.push(mount(ArchivePage, { target: document.body }), mount(DeleteIssues, { target: document.body }));
  await settle();
  return { api, a, b };
}
const buttons = () => [...document.querySelectorAll('button')];
const labelled = (t: string) => buttons().find((b) => b.textContent?.includes(t));

describe('the Archive page', () => {
  it('lists archived issues and searches them', async () => {
    const { a } = await setup();
    expect(document.querySelectorAll('li.row')).toHaveLength(2);
    const box = document.querySelector<HTMLInputElement>('input[placeholder="Search the archive"]')!;
    box.value = 'Older';
    box.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(document.querySelectorAll('li.row')).toHaveLength(1);
    expect(document.body.textContent).not.toContain(a.key + ' ');
  });

  it('restores the selected issues', async () => {
    const { api, a } = await setup();
    await run(api, 'archive.restore', '/archive', [a.id]);
    expect((await api.issues.get(a.id)).archived_at).toBeNull();
  });

  it('deletes only after the confirmation names the count, and Undo brings the issue back', async () => {
    const { api, a } = await setup();
    await run(api, 'archive.delete', '/archive', [a.id]);
    await settle();
    expect(document.body.textContent).toContain('Delete 1 issue permanently?');
    expect(document.body.textContent).toContain('cannot be undone later');
    await expect(api.issues.get(a.id)).resolves.toBeTruthy();
    labelled('Cancel')!.click();
    await settle();
    await expect(api.issues.get(a.id)).resolves.toBeTruthy();
    await run(api, 'archive.delete', '/archive', [a.id]);
    await settle();
    labelled('Delete 1 issue')!.click();
    await settle();
    await expect(api.issues.get(a.id)).rejects.toBeTruthy();
    expect((await api.history.undoStatus()).undo).toBe('Deleted 1 issue');
    await api.history.undo();
    expect((await api.issues.get(a.id)).title).toBe('Old thing');
  });

  it('empties the archive behind the same confirmation', async () => {
    const { api } = await setup();
    await run(api, 'archive.empty', '/archive', []);
    await settle();
    expect(document.body.textContent).toContain('Delete 2 issues permanently?');
    labelled('Delete 2 issues')!.click();
    await settle();
    expect(document.querySelectorAll('li.row')).toHaveLength(0);
    expect((await api.history.undoStatus()).undo).toBe('Emptied archive');
    await api.history.undo();
    expect(await api.views.query(archivedDef)).toHaveLength(2);
  });

  it('Delete permanently is offered on a live issue but never on the archive page', async () => {
    const api = createMockApi();
    const live = listCommands(ctx(api, '/my', [1]), { all: true }).find((c) => c.id === 'issue.deleteForever')!;
    expect(live.when!(ctx(api, '/my', [1]))).toBe(true);
    expect(live.when!(ctx(api, '/archive', [1]))).toBe(false);
    selection.clear();
  });
});
