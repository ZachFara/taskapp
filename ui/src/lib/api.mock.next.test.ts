import { describe, expect, it } from 'vitest';
import { createMockApi } from './api.mock';

describe('mock next actions (port of the core rules)', () => {
  it('an active project is stuck until an open next action is flagged; finishing it sticks it again; undo restores', async () => {
    const api = createMockApi();
    const p = await api.projects.create('fresh', 'Fresh', null, null);
    await api.projects.update({ ...p, status_id: 2 });
    const row = async () => (await api.next.list()).find((n) => n.project.id === p.id)!;
    expect((await row()).stuck).toBe(true);
    const a = await api.issues.create({ title: 'Call', project_id: p.id } as Parameters<typeof api.issues.create>[0]);
    await api.projects.setNext(p.id, a.id);
    expect(await row()).toMatchObject({ stuck: false, issue: { id: a.id } });
    expect((await api.history.undoStatus()).undo).toBe('Set next action');
    await api.issues.update(a.id, { status_id: 4 } as Parameters<typeof api.issues.update>[1]);
    expect((await row()).stuck).toBe(true);
    await api.history.undo();
    expect((await row()).stuck).toBe(false);
    await api.projects.setNext(p.id, null);
    expect((await row()).issue).toBeNull();
  });

  it('refuses an issue from another project', async () => {
    const api = createMockApi();
    const [p, q] = await Promise.all([api.projects.create('aa', 'A', null, null), api.projects.create('bb', 'B', null, null)]);
    const other = await api.issues.create({ title: 'x', project_id: q.id } as Parameters<typeof api.issues.create>[0]);
    await expect(api.projects.setNext(p.id, other.id)).rejects.toMatchObject({ kind: 'Invalid' });
  });
});
