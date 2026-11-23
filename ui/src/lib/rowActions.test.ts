import { describe, expect, it } from 'vitest';
import { setApi } from './api';
import { createMockApi, fixtures } from './api.mock';
import { patch } from './rowActions';

const def = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false } as const;

describe('row icon changes persist', () => {
  it('a_project_icon_picked_on_a_row_is_still_there_after_reload', async () => {
    const api = createMockApi(fixtures);
    setApi(api);
    const p = (await api.projects.list())[0];
    await patch({ kind: 'project', id: p.id, name: p.name }, { icon: 'icon:star', color: 'red' });
    const reloaded = (await api.projects.list()).find((x) => x.id === p.id);
    expect(reloaded?.icon).toBe('icon:star');
    expect(reloaded?.color).toBe('red');
  });

  it('a_view_icon_picked_on_a_row_is_still_there_after_reload', async () => {
    const api = createMockApi(fixtures);
    setApi(api);
    const v = await api.views.save(null, 'Alpha', def, null, null);
    await patch({ kind: 'view', id: v.id, name: v.name }, { icon: 'icon:flag', color: 'teal' });
    const reloaded = (await api.views.list()).find((x) => x.id === v.id);
    expect(reloaded?.icon).toBe('icon:flag');
    expect(reloaded?.color).toBe('teal');
    expect(reloaded?.name).toBe('Alpha');
  });
});
