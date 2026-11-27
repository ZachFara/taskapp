import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { api, setApi } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import ProjectsPage from './ProjectsPage.svelte';

const button = (root: ParentNode, text: string) => [...root.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === text)!;

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
  live = mount(ProjectsPage, { target });
  await vi.waitFor(() => expect(target.querySelector('table')).not.toBeNull());
  flushSync();
  return { mock, target };
}

describe('ProjectsPage', () => {
  it('shows the folders around the projects, nested and indented', async () => {
    const mock = createMockApi({ ...fixtures, viewFolders: [{ id: 1, name: 'Work', position: 0, section: 'project', parent_id: null }, { id: 2, name: 'Sub', position: 1, section: 'project', parent_id: 1 }] });
    const p = fixtures.projects[0];
    await mock.folders.placeItem('project', p.id, 2, null);
    const { target } = await open(mock);
    const rows = [...target.querySelectorAll('tbody tr')];
    const at = (sel: string) => rows.indexOf(target.querySelector(sel)!);
    expect([at('[data-folder-id="1"]'), at('[data-folder-id="2"]'), at(`[data-slug="${p.slug}"]`)].every((i, k, a) => i >= 0 && (k === 0 || i > a[k - 1]))).toBe(true);
    expect(target.querySelector<HTMLElement>('[data-folder-id="2"] td')!.style.paddingLeft).toContain('1 *');
    expect(target.querySelector<HTMLElement>(`[data-slug="${p.slug}"] td`)!.style.paddingLeft).toContain('2 *');
  });

  it('projects_table_has_a_row_per_project_and_timeline_link', async () => {
    const { target } = await open();
    const rows = target.querySelectorAll('tbody tr');
    expect(rows.length).toBe(fixtures.projects.length);
    expect(target.querySelector('a[href="/projects/timeline"]')).not.toBeNull();
  });

  it('a project without a target date says so in words, not a dash', async () => {
    const { target } = await open();
    const cells = [...target.querySelectorAll('tbody td.muted')].map((td) => td.textContent?.trim());
    expect(cells).toContain('No target');
    expect(cells).not.toContain('—');
  });

  it('new_project_is_created_and_listed', async () => {
    const { mock, target } = await open();
    const create = vi.spyOn(mock.projects, 'create');
    button(target, 'New project').click();
    flushSync();
    const name = document.querySelector<HTMLInputElement>('input[aria-label="Project name"]')!;
    name.value = 'Pricing refresh';
    name.dispatchEvent(new Event('input', { bubbles: true }));
    button(document, 'Create project').click();
    await vi.waitFor(() => expect(create).toHaveBeenCalledWith('pricing-refresh', 'Pricing refresh', null, null));
  });

  it('the command menu opens the new-project dialog', async () => {
    await open();
    window.dispatchEvent(new CustomEvent('command:project.new'));
    flushSync();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('New project');
  });
});

describe('ProjectsPage fields', () => {
  it('changes a project status in place without navigating', async () => {
    const { target } = await open();
    const project = fixtures.projects[0];
    const row = target.querySelector<HTMLElement>(`[data-slug="${project.slug}"]`)!;
    const originalPath = location.pathname;
    row.querySelector<HTMLButtonElement>(`button[aria-label="Change status of ${project.name}"]`)!.click();
    await vi.waitFor(() => expect(document.querySelector('[role="listbox"][aria-label="Project status"]')).not.toBeNull());
    const option = [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((o) => o.textContent?.trim() === 'Paused')!;
    option.click();
    await vi.waitFor(async () => expect((await api.projects.list()).find((p) => p.id === project.id)?.status_id).toBe(3));
    await vi.waitFor(() => expect(row.textContent).toContain('Paused'));
    expect(location.pathname).toBe(originalPath);
  });

  it('reports a failed status update without changing the displayed status', async () => {
    const mock = createMockApi(fixtures);
    const { target } = await open(mock);
    vi.spyOn(mock.projects, 'update').mockRejectedValueOnce(new Error('Save failed'));
    const project = fixtures.projects[0];
    const row = target.querySelector<HTMLElement>(`[data-slug="${project.slug}"]`)!;
    row.querySelector<HTMLButtonElement>(`button[aria-label="Change status of ${project.name}"]`)!.click();
    await vi.waitFor(() => expect(document.querySelector('[role="listbox"][aria-label="Project status"]')).not.toBeNull());
    [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((o) => o.textContent?.trim() === 'Paused')!.click();
    await vi.waitFor(() => expect(row.querySelector('[role="alert"]')?.textContent).toContain('Save failed'));
    expect(row.textContent).toContain('Planned');
  });

  it('has status, priority and label columns and a project-scoped property column', async () => {
    const mock = createMockApi(fixtures);
    await mock.properties.createDef({ key: 'sponsor', name: 'Sponsor', kind: 'Text', options: [], project_id: null, position: 0, applies_to: 'Projects' });
    const { target } = await open(mock);
    await vi.waitFor(() => expect([...target.querySelectorAll('th')].map((h) => h.textContent)).toContain('Sponsor'));
    const heads = [...target.querySelectorAll('thead th')].map((h) => h.textContent);
    expect(heads.slice(0, 4)).toEqual(['Project', 'Status', 'Priority', 'Labels']);
    expect([...target.querySelectorAll('tbody tr td:nth-child(2)')].map((c) => c.textContent?.trim())).toContain('Planned');
  });
});
