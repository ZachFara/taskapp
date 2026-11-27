import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listCommands, type Command, type Ctx } from './index';
import { createMockApi, fixtures } from '../api.mock';

const ids = async (api: ReturnType<typeof createMockApi>, n: number) => {
  const made = [];
  for (let i = 0; i < n; i++) made.push((await api.issues.create({ title: `ABC t${i}` })).id);
  return made;
};
const cmd = (id: string) => listCommands({} as Ctx, { all: true }).find((c) => c.id === id) as Command;
const itemsOf = async (c: Command, ctx: Ctx) => (await c.page!(ctx).items);
const labelsOf = async (c: Command, ctx: Ctx) => (await itemsOf(c, ctx)).map((i) => i.label);

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
});

const noView = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false } as const;

describe('async pages', () => {
  it('the first open lists the choices without any earlier load', async () => {
    const l = await api.labels.create('Fresh', 'red', null);
    const page = cmd('issue.labels').page!(ctx([1]));
    expect(page.items).toBeInstanceOf(Promise);
    expect((await page.items as { label: string }[]).map((i) => i.label)).toContain(`Add ${l.name}`);
  });

  it('a page with fixed rows lists them at once', () => {
    expect(Array.isArray(cmd('issue.priority').page!(ctx([1])).items)).toBe(true);
  });

  it('a page that cannot load rejects instead of listing nothing', async () => {
    const broken = { ...api, statuses: { list: async () => { throw new Error('offline'); } } } as unknown as Ctx['api'];
    await expect(cmd('issue.status').page!({ ...ctx([1]), api: broken }).items).rejects.toThrow('offline');
  });
});

describe('typed dates and estimates', () => {
  const page = (id: string) => cmd(id).page!(ctx([1, 2]));
  it('a typed day shows a preview that sets it', async () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-11-22T12:00:00Z'));
    await page('issue.due').items;
    const row = page('issue.due').parse!('fri')!;
    expect(row.label).toBe('Set the due date to Fri, Nov 27, 2026');
    await row.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { due: { Date: '2026-11-27' } } }));
    await page('issue.due').parse!('none')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { due: null } }));
    await page('issue.start').parse!('+2w')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { start_date: '2026-12-06' } }));
    expect(page('issue.due').parse!('soonish')).toBeNull();
    vi.useRealTimers();
  });

  it('the presets use the workspace zone for today', async () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-11-22T12:00:00Z'));
    const ahead = createMockApi({ ...fixtures, workspace: { setup_done: true, name: 'W', key: 'ABC', timezone: 'Pacific/Kiritimati' } });
    const c = { ...ctx([1]), api: ahead as unknown as Ctx['api'] };
    const items = (await cmd('issue.due').page!(c).items) as { label: string; run: () => Promise<void> }[];
    (ahead.issues as unknown as { bulkUpdate: unknown }).bulkUpdate = bulk;
    await items.find((i) => i.label === 'Today')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1], expect.objectContaining({ patch: { due: { Date: '2026-11-23' } } }));
    vi.useRealTimers();
  });

  it('a typed estimate shows a preview that sets the minutes', async () => {
    await expect(page('issue.estimate').parse!('1h30m')!.label).toBe('Set the estimate to 1h 30m');
    await page('issue.estimate').parse!('1.5h')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { estimate_minutes: 90 } }));
    await page('issue.estimate').parse!('none')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { estimate_minutes: null } }));
    expect(page('issue.estimate').parse!('lots')).toBeNull();
  });

  it('a bare number is hours', async () => {
    expect(page('issue.estimate').parse!('5')!.label).toBe('Set the estimate to 5h');
    await page('issue.estimate').parse!('2.5')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { estimate_minutes: 150 } }));
  });
});

describe('issue commands', () => {
  it('registers every action with its key', () => {
    const keys: Record<string, string> = {
      'issue.status': 's', 'issue.priority': 'p', 'issue.labels': ['shift+l', 'l'], 'issue.project': 'm', 'issue.cycle': 'shift+c',
      'issue.due': 'd', 'issue.start': 'shift+d', 'issue.estimate': 'e', 'issue.property': 'a', 'issue.relation': 'r',
      'issue.parent': 'shift+p', 'issue.pin': '.', 'issue.archive': '#', 'issue.copyKey': 'mod+c',
      'issue.copyLink': 'mod+shift+c', 'issue.duplicate': 'mod+d', 'issue.delete': 'delete',
    };
    for (const [id, key] of Object.entries(keys)) expect(cmd(id)?.shortcut, id).toEqual(key);
  });

  it('status page lists the statuses', async () => {
    const names = (await api.statuses.list()).map((s) => s.name);
    expect(await labelsOf(cmd('issue.status'), ctx([1]))).toEqual(names);
  });

  it('priority page lists the five priorities', async () => {
    expect(await labelsOf(cmd('issue.priority'), ctx([1]))).toEqual(['No priority', 'Urgent', 'High', 'Medium', 'Low']);
  });

  it('project, cycle and parent pages offer a way to clear', async () => {
    for (const id of ['issue.project', 'issue.cycle', 'issue.parent'])
      expect((await labelsOf(cmd(id), ctx([1])))[0], id).toMatch(/^No /);
  });

  it('labels page adds or removes each label', async () => {
    const l = await api.labels.create('bug', 'red', null);
    expect(await labelsOf(cmd('issue.labels'), ctx([1]))).toEqual(expect.arrayContaining([`Add ${l.name}`, `Remove ${l.name}`]));
  });

  it('property page lists select options and checkboxes', async () => {
    await api.properties.createDef({
      key: 'qa_area', name: 'QA area', kind: 'Select', options: [{ value: 'web', color: 'blue' }], project_id: null, position: 0,
    });
    await api.properties.createDef({ key: 'qa_ok', name: 'QA reviewed', kind: 'Checkbox', options: [], project_id: null, position: 1 });
    expect(await labelsOf(cmd('issue.property'), ctx([1]))).toEqual(expect.arrayContaining(['QA area: web', 'QA reviewed: yes', 'QA reviewed: no']));
  });

  it('relation page excludes the selected issues and offers each kind', async () => {
    const [a, b] = await ids(api, 2);
    const items = await labelsOf(cmd('issue.relation'), ctx([a]));
    expect(items.some((t) => t.startsWith('Blocks') && t.includes('ABC t1'))).toBe(true);
    expect(items.some((t) => t.includes('ABC t0'))).toBe(false);
    expect(b).toBeDefined();
  });

  it('runs one bulk call with all three selected ids', async () => {
    const sel = await ids(api, 3);
    const item = (await itemsOf(cmd('issue.priority'), ctx(sel))).find((i) => i.label === 'High')!;
    await item.run();
    expect(bulk).toHaveBeenCalledTimes(1);
    expect(bulk).toHaveBeenCalledWith(sel, expect.objectContaining({ patch: { priority: 2 } }));
  });

  it('falls back to the focused, then the open issue when nothing is selected', async () => {
    await (await itemsOf(cmd('issue.priority'), ctx([], { focusedIssueId: 7, openIssueId: 9 })))[1].run();
    expect(bulk.mock.calls[0][0]).toEqual([7]);
    await (await itemsOf(cmd('issue.priority'), ctx([], { openIssueId: 9 })))[1].run();
    expect(bulk.mock.calls[1][0]).toEqual([9]);
  });

  it('labels add and remove go in the patch', async () => {
    const l = await api.labels.create('bug', 'red', null);
    const items = await itemsOf(cmd('issue.labels'), ctx([1, 2]));
    await items.find((i) => i.label === 'Add bug')!.run();
    await items.find((i) => i.label === 'Remove bug')!.run();
    expect(bulk.mock.calls[0][1]).toMatchObject({ add_labels: [l.id] });
    expect(bulk.mock.calls[1][1]).toMatchObject({ remove_labels: [l.id] });
  });

  it('archive confirms a big selection through the command menu, never window.confirm', async () => {
    const confirm = vi.fn(() => true);
    vi.stubGlobal('confirm', confirm);
    const opened: unknown[] = [];
    const on = (e: Event) => opened.push((e as CustomEvent).detail);
    addEventListener('command:palette.open', on);
    await cmd('issue.archive').run!(ctx([1, 2, 3, 4, 5]));
    expect(bulk).toHaveBeenCalledTimes(1);
    expect(bulk.mock.calls[0][1]).toMatchObject({ archive: true });
    expect(opened).toEqual([]);
    await cmd('issue.archive').run!(ctx([1, 2, 3, 4, 5, 6]));
    expect(bulk).toHaveBeenCalledTimes(1);
    expect(opened).toEqual([{ page: 'issue.archive.confirm' }]);
    const page = cmd('issue.archive.confirm').page!(ctx([1, 2, 3, 4, 5, 6]));
    expect(page.title).toBe('Archive 6 issues?');
    const items = await page.items;
    await items[1].run();
    expect(bulk).toHaveBeenCalledTimes(1);
    await items[0].run();
    expect(bulk).toHaveBeenCalledTimes(2);
    expect(bulk.mock.calls[1]).toEqual([[1, 2, 3, 4, 5, 6], expect.objectContaining({ archive: true })]);
    expect(confirm).not.toHaveBeenCalled();
    removeEventListener('command:palette.open', on);
    vi.unstubAllGlobals();
  });

  it('the confirm step only lists for a big selection', () => {
    expect(cmd('issue.archive.confirm').when!(ctx([1, 2, 3]))).toBe(false);
    expect(cmd('issue.archive.confirm').when!(ctx([1, 2, 3, 4, 5, 6]))).toBe(true);
  });

  it('delete only archives', async () => {
    await cmd('issue.delete').run!(ctx([1]));
    expect(bulk.mock.calls[0][1]).toMatchObject({ archive: true });
  });

  it('duplicate copies the whole selection in one backend call', async () => {
    const sel = await ids(api, 3);
    const spy = vi.spyOn(api.issues, 'duplicate');
    await cmd('issue.duplicate').run!(ctx(sel));
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith(sel);
    expect((await api.views.query(noView)).filter((i) => i.title.startsWith('ABC t'))).toHaveLength(6);
  });

  it('pin toggles each selected issue', async () => {
    const [a, b] = await ids(api, 2);
    await api.pins.pin('Issue', (await api.issues.get(a)).key, null);
    await cmd('issue.pin').run!(ctx([a, b]));
    const targets = (await api.pins.listResolved()).map((r) => r.pin.target);
    expect(targets).toEqual([(await api.issues.get(b)).key]);
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

describe('entity pages draw each row with its own mark', () => {
  it('project, status, label and cycle rows carry their icon and colour', async () => {
    const { loadChoices } = await import('./defs/issue/_issue');
    await loadChoices(api as unknown as Parameters<typeof loadChoices>[0]);
    const byLabel = async (id: string, label: string) => (await itemsOf(cmd(id), ctx([1]))).find((i) => i.label === label);

    const [project] = await api.projects.list();
    expect((await byLabel('issue.project', project.name))?.entity).toEqual({ icon: project.icon, color: project.color });

    const [status] = await api.statuses.list();
    expect((await byLabel('issue.status', status.name))?.status).toEqual(status);

    await api.labels.create('Marked', 'red', 'icon:star');
    expect((await byLabel('issue.labels', 'Add Marked'))?.entity).toEqual({ icon: 'icon:star', color: 'red' });
    expect((await byLabel('issue.labels', 'Remove Marked'))?.entity).toEqual({ icon: 'icon:star', color: 'red' });

    const [cycle] = await api.cycles.list();
    const cycleRow = (await itemsOf(cmd('issue.cycle'), ctx([1]))).find((i) => i.label === (cycle.name ?? `Cycle ${cycle.number}`));
    expect(cycleRow?.entity).toEqual({ icon: cycle.icon, color: cycle.color });
  });
});
