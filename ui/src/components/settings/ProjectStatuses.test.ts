import { describe, expect, it } from 'vitest';
import ProjectStatuses from './ProjectStatuses.svelte';
import { clickLabel, field, mountSettings, press, setField } from './testing';

describe('ProjectStatuses', () => {
  it('lists the seeded statuses in order, each with its category glyph', async () => {
    const { target } = await mountSettings(ProjectStatuses);
    const rows = [...target.querySelectorAll('.es-row')];
    expect(rows.map((r) => (r.querySelector('input') as HTMLInputElement).value)).toEqual(['Planned', 'In progress', 'Paused', 'Done', 'Cancelled']);
    expect(rows.map((r) => r.querySelector('svg.glyph[data-category]')?.getAttribute('data-category'))).toEqual(['Backlog', 'Started', 'Unstarted', 'Completed', 'Canceled']);
  });

  it('adds a status once and renames it with one update', async () => {
    const { target, calls } = await mountSettings(ProjectStatuses);
    await setField(target, 'New project status name', 'Review');
    await press(target, 'Add status');
    expect(calls.filter((c) => c === 'project.statusesCreate')).toHaveLength(1);
    await setField(target, 'Name of Review', 'QA');
    expect(calls.filter((c) => c === 'project.statusesUpdate')).toHaveLength(1);
    expect(field(target, 'Name of QA')).toBeTruthy();
  });

  it('moves a status with one reorder call', async () => {
    const { calls, api } = await mountSettings(ProjectStatuses);
    await clickLabel('Move Paused up');
    expect(calls.filter((c) => c === 'project.statusesReorder')).toHaveLength(1);
    expect((await api.project.statusesList()).map((s) => s.name).slice(0, 3)).toEqual(['Planned', 'Paused', 'In progress']);
  });

  it('delete needs a target and moves the projects there', async () => {
    const { api } = await mountSettings(ProjectStatuses);
    const before = (await api.projects.list()).filter((p) => p.status_id === 1).length;
    expect(before).toBeGreaterThan(0);
    await clickLabel('Delete Planned');
    expect(document.querySelector<HTMLButtonElement>('button.danger, [data-variant="danger"]')?.disabled ?? true).toBe(true);
    await clickLabel('Move projects to Done');
    const confirm = [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Delete status')!;
    confirm.click();
    await new Promise((r) => setTimeout(r, 0));
    expect((await api.project.statusesList()).map((s) => s.name)).not.toContain('Planned');
    expect((await api.projects.list()).every((p) => p.status_id !== 1)).toBe(true);
  });
});
