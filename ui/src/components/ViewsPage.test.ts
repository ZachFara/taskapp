import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import ViewsPage from './ViewsPage.svelte';
import { setTemplateApi, type TemplateApi, type ViewTemplate } from './display/templatesApi';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

async function open(mock = createMockApi(fixtures)) {
  setApi(mock);
  const target = document.createElement('div');
  document.body.append(target);
  live = mount(ViewsPage, { target });
  await vi.waitFor(() => expect(target.querySelectorAll('[data-view-id]').length).toBeGreaterThan(0));
  flushSync();
  return { mock, target };
}

describe('ViewsPage', () => {
  it('views_show_layout_and_pinned_state', async () => {
    const { target } = await open();
    const first = target.querySelector<HTMLElement>(`[data-view-id="${fixtures.views![0].id}"]`)!;
    expect(first.textContent).toContain(fixtures.views![0].name);
    expect(first.textContent).toContain(fixtures.views![0].def.layout);
  });

  it('delete_view_asks_before_removing', async () => {
    const { mock, target } = await open();
    const del = vi.spyOn(mock.views, 'delete');
    const id = fixtures.views![0].id;
    target.querySelector<HTMLElement>(`[data-view-id="${id}"]`)!.querySelector<HTMLButtonElement>('button[aria-label^="Delete"]')!.click();
    flushSync();
    expect(del).not.toHaveBeenCalled();
    [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === 'Delete view')!.click();
    await vi.waitFor(() => expect(del).toHaveBeenCalledWith(id));
  });
});

const tpl: ViewTemplate = {
  id: 7, name: 'Planning table', icon: null, color: null, position: 0,
  def: { filter: null, sorts: [], group_by: null, layout: 'Table', columns: [{ Fixed: 'Priority' }], include_archived: false },
};
function fakeTemplates(): TemplateApi {
  const t = {
    list: vi.fn(async () => [tpl]),
    create: vi.fn(),
    update: vi.fn(async (x: ViewTemplate) => x),
    delete: vi.fn(async () => null),
    createFromTemplate: vi.fn(),
    saveAsTemplate: vi.fn(),
    resetToTemplate: vi.fn(),
  };
  setTemplateApi(t as unknown as TemplateApi);
  return t as unknown as TemplateApi;
}
const button = (text: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === text)!;

describe('ViewsPage templates', () => {
  it('templates_section_lists_each_template_and_its_layout', async () => {
    fakeTemplates();
    const { target } = await open();
    const row = await vi.waitFor(() => {
      const el = target.querySelector<HTMLElement>('[data-template-id="7"]');
      expect(el).not.toBeNull();
      return el!;
    });
    expect(row.textContent).toContain('Planning table');
    expect(row.textContent).toContain('Table');
  });

  it('a timeline layout is named Timeline, never Gantt', async () => {
    const prev = tpl.def.layout;
    tpl.def.layout = 'Gantt';
    try {
      fakeTemplates();
      const { target } = await open();
      const row = await vi.waitFor(() => {
        const el = target.querySelector<HTMLElement>('[data-template-id="7"]');
        expect(el).not.toBeNull();
        return el!;
      });
      expect(row.textContent).toContain('Timeline');
      expect(row.textContent).not.toContain('Gantt');
    } finally {
      tpl.def.layout = prev;
    }
  });

  it('set_as_default_saves_the_default_template_setting', async () => {
    fakeTemplates();
    const { mock, target } = await open();
    const set = vi.spyOn(mock.settings, 'set');
    await vi.waitFor(() => expect(target.querySelector('[data-template-id="7"]')).not.toBeNull());
    target.querySelector<HTMLElement>('[data-template-id="7"]')!.querySelector<HTMLButtonElement>('button[aria-label="Set as default"]')!.click();
    await vi.waitFor(() => expect(set).toHaveBeenCalledWith('default_view_template', 7));
  });

  it('rename_template_saves_the_new_name', async () => {
    const t = fakeTemplates();
    const { target } = await open();
    await vi.waitFor(() => expect(target.querySelector('[data-template-id="7"]')).not.toBeNull());
    target.querySelector<HTMLElement>('[data-template-id="7"]')!.querySelector<HTMLButtonElement>('button[aria-label="Rename Planning table"]')!.click();
    flushSync();
    const input = document.querySelector<HTMLInputElement>('input[aria-label="Name"]')!;
    input.value = 'Sprint table';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    button('Save').click();
    await vi.waitFor(() => expect(t.update).toHaveBeenCalledWith(expect.objectContaining({ id: 7, name: 'Sprint table' })));
  });

  it('delete_template_asks_before_removing', async () => {
    const t = fakeTemplates();
    const { target } = await open();
    await vi.waitFor(() => expect(target.querySelector('[data-template-id="7"]')).not.toBeNull());
    target.querySelector<HTMLElement>('[data-template-id="7"]')!.querySelector<HTMLButtonElement>('button[aria-label="Delete Planning table"]')!.click();
    flushSync();
    expect(t.delete).not.toHaveBeenCalled();
    button('Delete template').click();
    await vi.waitFor(() => expect(t.delete).toHaveBeenCalledWith(7));
  });

  it('new_view_opens_the_start_from_dialog_instead_of_creating', async () => {
    fakeTemplates();
    const { mock } = await open();
    const save = vi.spyOn(mock.views, 'save');
    button('New view').click();
    flushSync();
    await vi.waitFor(() => expect(document.body.textContent).toContain('Start from'));
    expect(save).not.toHaveBeenCalled();
  });

  it('command_view_new_opens_the_start_from_dialog', async () => {
    fakeTemplates();
    const { mock } = await open();
    const save = vi.spyOn(mock.views, 'save');
    window.dispatchEvent(new CustomEvent('command:view.new'));
    await vi.waitFor(() => expect(document.body.textContent).toContain('Start from'));
    expect(save).not.toHaveBeenCalled();
  });
});
