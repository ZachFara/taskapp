import { describe, expect, it } from 'vitest';
import { setApi } from '../api';
import { createMockApi, fixtures } from '../api.mock';
import { rowUi } from '../rowUi.svelte';
import { commandById, type Ctx } from './index';

const mock = () => {
  const api = createMockApi({ ...fixtures, viewFolders: [{ id: 1, name: 'Work', position: 0, section: 'view', parent_id: null }, { id: 2, name: 'Sub', position: 1, section: 'view', parent_id: 1 }, { id: 3, name: 'Mine', position: 2, section: 'project', parent_id: null }] });
  setApi(api);
  return api;
};
const ctx = (api: Ctx['api'], route: string): Ctx => ({ route, selection: [], focusedIssueId: null, openIssueId: null, api, navigate: () => {} });

describe('folder commands', () => {
  it('move to folder lists the folders of the open item section by path, and moves it', async () => {
    const api = mock();
    const c = ctx(api, '/view/2');
    const cmd = commandById('item.moveToFolder')!;
    expect(cmd.when!(c)).toBe(true);
    expect(cmd.when!(ctx(api, '/projects'))).toBe(false);
    const items = await cmd.page!(c).items;
    expect(items.map((i) => i.label)).toEqual(['Top level', 'Work', 'Work / Sub', 'New folder…']);
    await items[2].run();
    expect((await api.views.list()).find((v) => v.id === 2)!.folder_id).toBe(2);
    await (await cmd.page!(c).items)[0].run();
    expect((await api.views.list()).find((v) => v.id === 2)!.folder_id).toBeNull();
    const slug = fixtures.projects[0].slug;
    expect((await cmd.page!(ctx(api, `/project/${slug}`)).items).map((i) => i.label)).toEqual(['Top level', 'Mine', 'New folder…']);
  });

  it('new folder from the command menu goes to the section of the page, then asks for a name', async () => {
    const api = mock();
    await commandById('folder.new')!.run!(ctx(api, '/projects'));
    expect((await api.folders.list()).at(-1)).toMatchObject({ section: 'project', parent_id: null });
    await commandById('folder.new')!.run!(ctx(api, '/projects'), 1, { section: 'cycle' });
    expect((await api.folders.list()).at(-1)).toMatchObject({ section: 'cycle' });
  });

  it('new_folder_makes_one_and_asks_for_its_name', async () => {
    const api = mock();
    await commandById('folder.new')!.run!(ctx(api, '/views'));
    expect((await api.folders.list()).map((f) => f.name)).toEqual(['Work', 'Sub', 'Mine', 'New folder']);
    expect(rowUi.dialog).toMatchObject({ mode: 'rename', target: { kind: 'folder', id: 4 } });
  });
});
