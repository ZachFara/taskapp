import { beforeEach, describe, expect, it } from 'vitest';
import { listCommands, type Command, type Ctx } from './index';
import { setApi } from '../api';
import { createMockApi } from '../api.mock';

const cmd = (id: string) => listCommands({} as Ctx, { all: true }).find((c) => c.id === id) as Command;
let api: ReturnType<typeof createMockApi>;
const ctx = (route = '/project/website'): Ctx => ({ route, selection: [], focusedIssueId: null, openIssueId: null, api: api as unknown as Ctx['api'], navigate: () => {} });
const website = async () => (await api.projects.list()).find((p) => p.slug === 'website')!;
const pick = async (id: string, label: string) => (await cmd(id).page!(ctx()).items).find((i) => i.label === label)!.run();

beforeEach(() => {
  api = createMockApi();
  setApi(api as never);
});

describe('project field commands', () => {
  it('are offered on a project page only', () => {
    for (const id of ['project.status', 'project.priority', 'project.labels', 'project.property']) {
      expect(cmd(id).when!(ctx()), id).toBe(true);
      expect(cmd(id).when!(ctx('/projects')), id).toBe(false);
    }
  });

  it('Set project status lists the project statuses and sets the chosen one', async () => {
    const labels = (await cmd('project.status').page!(ctx()).items).map((i) => i.label);
    expect(labels).toEqual(['Planned', 'In progress', 'Paused', 'Done', 'Cancelled']);
    await pick('project.status', 'Paused');
    expect((await website()).status_id).toBe(3);
  });

  it('Set project priority sets it', async () => {
    await pick('project.priority', 'High');
    expect((await website()).priority).toBe(2);
  });

  it('labels add and remove', async () => {
    await pick('project.labels', 'Add Bug');
    expect((await website()).label_ids).toEqual([1]);
    await pick('project.labels', 'Remove Bug');
    expect((await website()).label_ids).toEqual([]);
  });

  it('a select property that applies to projects can be set; issue-only ones are not listed', async () => {
    await api.properties.createDef({ key: 'tier', name: 'Tier', kind: 'Select', options: [{ value: 'gold', color: 'amber' }], project_id: null, position: 0, applies_to: 'Projects' });
    expect((await cmd('project.property').page!(ctx()).items).map((i) => i.label)).toEqual(['Tier: gold']);
    await pick('project.property', 'Tier: gold');
    expect((await website()).properties).toEqual({ tier: 'gold' });
  });
});
