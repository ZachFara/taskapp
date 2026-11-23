import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi } from '../lib/api';
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
