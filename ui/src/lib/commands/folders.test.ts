import { describe, expect, it } from 'vitest';
import { setApi } from '../api';
import { createMockApi, fixtures } from '../api.mock';
import { rowUi } from '../rowUi.svelte';
import { commandById, type Ctx } from './index';

const mock = () => {
  const api = createMockApi({ ...fixtures, viewFolders: [{ id: 1, name: 'Work', position: 0 }] });
  setApi(api);
  return api;
};
const ctx = (api: Ctx['api'], route: string): Ctx => ({ route, selection: [], focusedIssueId: null, openIssueId: null, api, navigate: () => {} });

describe('folder commands', () => {
  it('move_view_to_folder lists the folders and the top level, and moves the open view', async () => {
    const api = mock();
    const c = ctx(api, '/view/2');
    const cmd = commandById('view.moveToFolder')!;
    expect(cmd.when!(c)).toBe(true);
    expect(cmd.when!(ctx(api, '/projects'))).toBe(false);
    const items = await cmd.page!(c).items;
    expect(items.map((i) => i.label)).toEqual(['No folder (top level)', 'Work']);
    await items[1].run();
    expect((await api.views.list()).find((v) => v.id === 2)!.folder_id).toBe(1);
    await (await cmd.page!(c).items)[0].run();
    expect((await api.views.list()).find((v) => v.id === 2)!.folder_id).toBeNull();
  });

  it('new_folder_makes_one_and_asks_for_its_name', async () => {
    const api = mock();
    await commandById('folder.new')!.run!(ctx(api, '/views'));
    expect((await api.folders.list()).map((f) => f.name)).toEqual(['Work', 'New folder']);
    expect(rowUi.dialog).toMatchObject({ mode: 'rename', target: { kind: 'folder', id: 2 } });
  });
});
