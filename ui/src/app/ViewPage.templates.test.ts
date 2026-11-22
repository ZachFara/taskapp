import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import type { SavedView, ViewDef } from '../lib/bindings';
import { setTemplateApi, type TemplateApi } from '../components/display/templatesApi';
import ViewPage from './ViewPage.svelte';

vi.setConfig({ testTimeout: 30000 });
afterEach(() => {
  cleanup();
  setTemplateApi(null);
});

const def: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false };
function fake(): TemplateApi {
  const t = {
    list: vi.fn(async () => []),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    createFromTemplate: vi.fn(),
    saveAsTemplate: vi.fn(async () => ({ id: 3, name: 'Mine', icon: null, color: null, position: 0, def })),
    resetToTemplate: vi.fn(async (id: number) => ({ id, name: 'Mine', def, position: 0, icon: null, color: null })),
  };
  setTemplateApi(t as unknown as TemplateApi);
  return t as unknown as TemplateApi;
}
async function open(extra: Partial<SavedView> & { template_id?: number | null } = {}) {
  const v = { id: 1, name: 'Mine', def, position: 0, icon: null, color: null, ...extra } as SavedView;
  const mock: Api = createMockApi({ ...fixtures, views: [v] });
  setApi(mock);
  render(ViewPage, { params: { id: '1' } });
  await screen.findByRole('heading', { level: 1 });
  return mock;
}

describe('ViewPage templates', () => {
  it('save_as_template_command_saves_this_view_as_a_template', async () => {
    const t = fake();
    await open();
    window.dispatchEvent(new CustomEvent('command:view.saveAsTemplate'));
    await vi.waitFor(() => expect(t.saveAsTemplate).toHaveBeenCalledWith(1, null));
  });

  it('reset_to_template_command_resets_a_view_that_came_from_one', async () => {
    const t = fake();
    await open({ template_id: 7 } as never);
    window.dispatchEvent(new CustomEvent('command:view.resetToTemplate'));
    await vi.waitFor(() => expect(t.resetToTemplate).toHaveBeenCalledWith(1));
  });

  it('more_menu_hides_reset_to_template_without_a_template', async () => {
    const user = userEvent.setup();
    fake();
    await open();
    await user.click(screen.getByRole('button', { name: 'More' }));
    expect(await screen.findByText('Save as template')).toBeTruthy();
    expect(screen.queryByText('Reset to template')).toBeNull();
  });
});
