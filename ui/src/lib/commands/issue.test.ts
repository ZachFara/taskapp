import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listCommands, type Command, type Ctx } from './index';
import { loadChoices } from './defs/issue/_issue';
import { createMockApi } from '../api.mock';

const ids = async (api: ReturnType<typeof createMockApi>, n: number) => {
  const made = [];
  for (let i = 0; i < n; i++) made.push((await api.issues.create({ title: `ABC t${i}` })).id);
  return made;
};
const cmd = (id: string) => listCommands({} as Ctx, { all: true }).find((c) => c.id === id) as Command;
const labelsOf = (c: Command, ctx: Ctx) => c.page!(ctx).items.map((i) => i.label);

let api: ReturnType<typeof createMockApi>;
let bulk: ReturnType<typeof vi.fn>;
let ctx: (selection: number[], over?: Partial<Ctx>) => Ctx;
beforeEach(async () => {
  api = createMockApi();
  bulk = vi.fn(async () => null);
  (api.issues as unknown as { bulkUpdate: unknown }).bulkUpdate = bulk;
  ctx = (selection, over = {}) => ({
    route: '/', selection, focusedIssueId: null, openIssueId: null, api: api as unknown as Ctx['api'], navigate: () => {}, ...over,
  });
  await loadChoices(api as unknown as Ctx['api']);
});

describe('issue commands', () => {
  it('registers every action with its key', () => {
    const keys: Record<string, string> = {
      'issue.status': 's', 'issue.priority': 'p', 'issue.labels': 'shift+l', 'issue.project': 'm', 'issue.cycle': 'shift+c',
      'issue.due': 'd', 'issue.start': 'shift+d', 'issue.estimate': 'e', 'issue.property': 'a', 'issue.relation': 'r',
      'issue.parent': 'shift+p', 'issue.pin': '.', 'issue.archive': '#', 'issue.copyKey': 'mod+c',
      'issue.copyLink': 'mod+shift+c', 'issue.duplicate': 'mod+d', 'issue.delete': 'delete',
    };
    for (const [id, key] of Object.entries(keys)) expect(cmd(id)?.shortcut, id).toBe(key);
  });

  it('status page lists the statuses', async () => {
    const names = (await api.statuses.list()).map((s) => s.name);
    expect(labelsOf(cmd('issue.status'), ctx([1]))).toEqual(names);
  });

  it('priority page lists the five priorities', () => {
    expect(labelsOf(cmd('issue.priority'), ctx([1]))).toEqual(['No priority', 'Urgent', 'High', 'Medium', 'Low']);
  });

  it('project, cycle and parent pages offer a way to clear', () => {
    for (const id of ['issue.project', 'issue.cycle', 'issue.parent'])
      expect(labelsOf(cmd(id), ctx([1]))[0], id).toMatch(/^No /);
  });

  it('labels page adds or removes each label', async () => {
    const l = await api.labels.create('bug', 'red', null);
    await loadChoices(api as unknown as Ctx['api']);
    expect(labelsOf(cmd('issue.labels'), ctx([1]))).toEqual(expect.arrayContaining([`Add ${l.name}`, `Remove ${l.name}`]));
  });

  it('property page lists select options and checkboxes', async () => {
    await api.properties.createDef({
      key: 'qa_area', name: 'QA area', kind: 'Select', options: [{ value: 'web', color: 'blue' }], project_id: null, position: 0,
    });
    await api.properties.createDef({ key: 'qa_ok', name: 'QA reviewed', kind: 'Checkbox', options: [], project_id: null, position: 1 });
    await loadChoices(api as unknown as Ctx['api']);
    expect(labelsOf(cmd('issue.property'), ctx([1]))).toEqual(expect.arrayContaining(['QA area: web', 'QA reviewed: yes', 'QA reviewed: no']));
  });

  it('relation page excludes the selected issues and offers each kind', async () => {
    const [a, b] = await ids(api, 2);
    await loadChoices(api as unknown as Ctx['api']);
    const items = labelsOf(cmd('issue.relation'), ctx([a]));
    expect(items.some((t) => t.startsWith('Blocks') && t.includes('ABC t1'))).toBe(true);
    expect(items.some((t) => t.includes('ABC t0'))).toBe(false);
    expect(b).toBeDefined();
  });

  it('runs one bulk call with all three selected ids', async () => {
    const sel = await ids(api, 3);
    const item = cmd('issue.priority').page!(ctx(sel)).items.find((i) => i.label === 'High')!;
    await item.run();
    expect(bulk).toHaveBeenCalledTimes(1);
    expect(bulk).toHaveBeenCalledWith(sel, expect.objectContaining({ patch: { priority: 3 } }));
  });

  it('falls back to the focused, then the open issue when nothing is selected', async () => {
    await cmd('issue.priority').page!(ctx([], { focusedIssueId: 7, openIssueId: 9 })).items[1].run();
    expect(bulk.mock.calls[0][0]).toEqual([7]);
    await cmd('issue.priority').page!(ctx([], { openIssueId: 9 })).items[1].run();
    expect(bulk.mock.calls[1][0]).toEqual([9]);
  });

  it('labels add and remove go in the patch', async () => {
    const l = await api.labels.create('bug', 'red', null);
    await loadChoices(api as unknown as Ctx['api']);
    const page = cmd('issue.labels').page!(ctx([1, 2]));
    await page.items.find((i) => i.label === 'Add bug')!.run();
    await page.items.find((i) => i.label === 'Remove bug')!.run();
    expect(bulk.mock.calls[0][1]).toMatchObject({ add_labels: [l.id] });
    expect(bulk.mock.calls[1][1]).toMatchObject({ remove_labels: [l.id] });
  });

  it('archive asks to confirm only for more than 5 issues', async () => {
    const confirm = vi.fn(() => false);
    vi.stubGlobal('confirm', confirm);
    await cmd('issue.archive').run!(ctx([1, 2, 3, 4, 5]));
    expect(confirm).not.toHaveBeenCalled();
    expect(bulk).toHaveBeenCalledTimes(1);
    expect(bulk.mock.calls[0][1]).toMatchObject({ archive: true });
    await cmd('issue.archive').run!(ctx([1, 2, 3, 4, 5, 6]));
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(bulk).toHaveBeenCalledTimes(1);
    confirm.mockReturnValue(true);
    await cmd('issue.archive').run!(ctx([1, 2, 3, 4, 5, 6]));
    expect(bulk).toHaveBeenCalledTimes(2);
    vi.unstubAllGlobals();
  });

  it('delete only archives', async () => {
    await cmd('issue.delete').run!(ctx([1]));
    expect(bulk.mock.calls[0][1]).toMatchObject({ archive: true });
  });

  it('copy key writes the key of the focused issue', async () => {
    const [a] = await ids(api, 1);
    const writeText = vi.fn(async () => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    await cmd('issue.copyKey').run!(ctx([], { focusedIssueId: a }));
    expect(writeText).toHaveBeenCalledWith((await api.issues.get(a)).key);
    await cmd('issue.copyLink').run!(ctx([], { focusedIssueId: a }));
    expect(writeText).toHaveBeenLastCalledWith(`taskapp://issue/${(await api.issues.get(a)).key}`);
    vi.unstubAllGlobals();
  });
});
