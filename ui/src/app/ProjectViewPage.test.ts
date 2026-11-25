import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import type { SavedView } from '../lib/bindings';
import NewViewDialog from '../components/display/NewViewDialog.svelte';
import Sidebar from '../components/Sidebar.svelte';
import ViewsPage from '../components/ViewsPage.svelte';
import { refreshAll } from '../lib/stores.svelte';
import ViewPage from './ViewPage.svelte';

vi.setConfig({ testTimeout: 30000 });
const base = { filter: null, sorts: [], group_by: null, layout: 'List' as const, columns: [], include_archived: false };
const mk = (id: number, name: string, entity: 'issue' | 'project'): SavedView => ({ id, name, entity, def: base, position: id, icon: null, color: null, template_id: null, folder_id: null });
const seed = () => ({ ...fixtures, views: [mk(1, 'Issues one', 'issue'), mk(2, 'Projects one', 'project')] });
afterEach(() => {
  cleanup();
  document.body.innerHTML = '';
});

describe('a project view page', () => {
  it('lists projects and offers project fields in the filter, group and sort pickers', async () => {
    setApi(createMockApi(seed()));
    await refreshAll();
    const user = userEvent.setup();
    const { container } = render(ViewPage, { params: { id: '2' } });
    await waitFor(() => expect(container.querySelectorAll('[data-slug]').length).toBe(fixtures.projects.length));
    await user.click(screen.getByRole('button', { name: 'Add filter' }));
    await waitFor(() => expect(document.querySelectorAll('[role="menu"] *').length).toBeGreaterThan(4));
    const items = [...document.querySelectorAll('[role="menu"] *')].filter((e) => !e.children.length).map((e) => e.textContent?.trim());
    expect(items).toEqual(expect.arrayContaining(['Status', 'Priority', 'Target', 'Progress', 'Issues']));
    expect(items).not.toContain('Project');
    expect(items).not.toContain('Due');
  });

  it('a filter chip narrows the projects', async () => {
    const s = seed();
    s.views[1].def = { ...base, filter: { Cond: { field: { Fixed: 'Title' }, op: 'Contains', value: 'mobile' } } };
    setApi(createMockApi(s));
    await refreshAll();
    const { container } = render(ViewPage, { params: { id: '2' } });
    await waitFor(() => expect([...container.querySelectorAll('[data-slug]')].map((e) => (e as HTMLElement).dataset.slug)).toEqual(['mobile']));
  });
});

describe('where project views show', () => {
  it('the sidebar lists them among the views, not under Projects', async () => {
    setApi(createMockApi(seed()));
    const live = mount(Sidebar, { target: document.body });
    const under = (label: string) => [...document.querySelectorAll(`[aria-label="${label}"] a`)].map((a) => a.textContent?.trim());
    await vi.waitFor(() => expect(under('Views')).toEqual(expect.arrayContaining(['Issues one', 'Projects one'])));
    expect(under('Projects')).not.toContain('Projects one');
    unmount(live);
  });

  it('the Views page lists both kinds and its Project views tab shows only project views', async () => {
    setApi(createMockApi(seed()));
    const live = mount(ViewsPage, { target: document.body });
    await vi.waitFor(() => expect(document.body.textContent).toContain('Projects one'));
    expect(document.body.textContent).toContain('Issues one');
    [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent?.trim() === 'Project views')!.click();
    flushSync();
    await vi.waitFor(() => expect(document.body.textContent).not.toContain('Issues one'));
    expect(document.body.textContent).toContain('Projects one');
    unmount(live);
  });

  it('the new view dialog creates a project view when Projects is chosen', async () => {
    const mock = createMockApi(seed());
    setApi(mock);
    const user = userEvent.setup();
    render(NewViewDialog, { open: true });
    await user.click(await screen.findByRole('tab', { name: 'Projects' }));
    await user.click(await screen.findByRole('button', { name: /Create view/ }));
    await waitFor(async () => expect((await mock.views.list()).some((v) => v.entity === 'project' && v.name === 'Untitled view')).toBe(true));
  });
});
