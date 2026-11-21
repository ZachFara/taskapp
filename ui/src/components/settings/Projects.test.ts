import { describe, expect, it } from 'vitest';
import Projects from './Projects.svelte';
import { mountSettings, press, setField } from './testing';

describe('Projects', () => {
  it('lists projects from the store', async () => {
    const { target } = await mountSettings(Projects);
    expect(target.textContent).toContain('Home');
    expect(target.textContent).toContain('Work');
  });

  it('creates a project', async () => {
    const { target, calls } = await mountSettings(Projects);
    await setField(target, 'Project slug', 'garden');
    await setField(target, 'Project name', 'Garden');
    await press(target, 'Add project');
    expect(calls).toContain('projects.create');
    expect(target.textContent).toContain('Garden');
  });
});
