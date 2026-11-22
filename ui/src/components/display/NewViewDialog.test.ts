import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import type { SavedView, ViewDef } from '../../lib/bindings';
import NewViewDialog from './NewViewDialog.svelte';
import { setTemplateApi, type TemplateApi, type ViewTemplate } from './templatesApi';

const planning: ViewTemplate = {
  id: 7,
  name: 'Planning table',
  icon: null,
  color: null,
  position: 0,
  def: {
    filter: null, sorts: [], group_by: null, layout: 'Table', include_archived: false,
    columns: [{ Fixed: 'Priority' }, { Fixed: 'Status' }, { Property: 'Owner' }],
  },
};

let fake: TemplateApi;
let mock: Api;
beforeEach(() => {
  mock = createMockApi(fixtures);
  setApi(mock);
  fake = {
    list: vi.fn(async () => [planning]),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    createFromTemplate: vi.fn(async (id: number, name: string | null) => {
      const v: SavedView = { id: 99, name: name ?? planning.name, def: structuredClone(planning.def) as ViewDef, position: 0, icon: null, color: null };
      return v;
    }),
    saveAsTemplate: vi.fn(),
    resetToTemplate: vi.fn(),
  } as unknown as TemplateApi;
  setTemplateApi(fake);
  history.replaceState(null, '', '/views');
});
afterEach(() => {
  cleanup();
  setTemplateApi(null);
});

describe('NewViewDialog', () => {
  it('new_view_offers_blank_and_each_template', async () => {
    render(NewViewDialog, { open: true });
    await screen.findByRole('button', { name: /Planning table/ });
    expect(screen.getByRole('button', { name: /Blank/ })).toBeTruthy();
  });

  it('new_view_from_template_creates_the_template_layout_and_fields', async () => {
    const user = userEvent.setup();
    render(NewViewDialog, { open: true });
    await user.click(await screen.findByRole('button', { name: /Planning table/ }));
    await user.click(screen.getByRole('button', { name: 'Create view' }));
    await waitFor(() => expect(fake.createFromTemplate).toHaveBeenCalledWith(7, null));
    expect(location.pathname).toBe('/view/99');
    const made = await (fake.createFromTemplate as any).mock.results[0].value;
    expect(made.def.layout).toBe('Table');
    expect(made.def.columns).toHaveLength(3);
  });

  it('blank_creates_a_plain_list_view_without_a_template', async () => {
    const user = userEvent.setup();
    const save = vi.spyOn(mock.views, 'save');
    render(NewViewDialog, { open: true });
    await user.click(await screen.findByRole('button', { name: /Blank/ }));
    await user.click(screen.getByRole('button', { name: 'Create view' }));
    await waitFor(() => expect(save).toHaveBeenCalled());
    expect(save.mock.calls[0][2]).toMatchObject({ layout: 'List', columns: [] });
    expect(fake.createFromTemplate).not.toHaveBeenCalled();
  });

  it('default_template_is_preselected', async () => {
    vi.spyOn(mock.settings, 'get').mockImplementation(async (key: string) => (key === 'default_view_template' ? 7 : null));
    render(NewViewDialog, { open: true });
    const btn = await screen.findByRole('button', { name: /Planning table/ });
    await waitFor(() => expect(btn.getAttribute('aria-pressed')).toBe('true'));
  });
});
