import { describe, expect, it } from 'vitest';
import Projects from './Projects.svelte';
import { clickLabel, field, mountSettings, press, setField } from './testing';

async function add(target: Element, slug: string, name: string) {
  await setField(target, 'Project slug', slug);
  await setField(target, 'Project name', name);
  await press(target, 'Add project');
}

describe('Projects', () => {
  it('creates a project once and shows it', async () => {
    const { target, calls } = await mountSettings(Projects);
    await add(target, 'garden', 'Garden');
    expect(calls.filter((c) => c === 'projects.create')).toHaveLength(1);
    expect(field(target, 'Name of Garden')).toBeTruthy();
    expect(target.textContent).toContain('garden');
  });

  it('renames inline with one call, keeping the other fields', async () => {
    const { target, calls, api } = await mountSettings(Projects);
    await add(target, 'garden', 'Garden');
    await setField(target, 'Name of Garden', 'Yard');
    expect(calls.filter((c) => c === 'projects.update')).toHaveLength(1);
    expect((await api.projects.list()).find((p) => p.slug === 'garden')?.name).toBe('Yard');
  });

  it('picker saves icon and colour with one call', async () => {
    const { target, calls, api } = await mountSettings(Projects);
    await add(target, 'garden', 'Garden');
    await clickLabel('Icon and colour of Garden');
    await clickLabel('teal');
    expect(calls.filter((c) => c === 'projects.update')).toHaveLength(1);
    expect((await api.projects.list()).find((p) => p.slug === 'garden')?.color).toBe('teal');
  });

  it('delete confirms first', async () => {
    const { target, calls } = await mountSettings(Projects);
    await add(target, 'garden', 'Garden');
    await clickLabel('Delete Garden');
    expect(calls).not.toContain('projects.delete');
    await press(document.body, 'Delete project');
    expect(calls.filter((c) => c === 'projects.delete')).toHaveLength(1);
  });
});
