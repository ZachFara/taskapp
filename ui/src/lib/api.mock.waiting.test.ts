import { describe, expect, it } from 'vitest';
import { createMockApi } from './api.mock';

describe('mock waiting list', () => {
  it('lists open waiting issues, due follow-ups first, and leaves out done and archived ones', async () => {
    const mock = createMockApi();
    const make = (title: string) => mock.issues.create({ title });
    const later = await make('later');
    const undated = await make('undated');
    const due = await make('due');
    const done = await make('done');
    const archived = await make('archived');
    await make('plain');
    const set = (id: number, patch: Record<string, unknown>) => mock.issues.update(id, patch);
    await set(later.id, { waiting_on: 'Ana', follow_up: '2999-01-01' });
    await set(undated.id, { waiting_on: 'Bo' });
    await set(due.id, { waiting_on: 'Cy', follow_up: '2000-01-01' });
    await set(done.id, { waiting_on: 'Di', follow_up: '2000-01-01' });
    await set(archived.id, { waiting_on: 'Ed' });
    const completed = (await mock.statuses.list()).find((s) => s.category === 'Completed')!;
    await set(done.id, { status_id: completed.id });
    await mock.issues.archive(archived.id);
    expect((await mock.waiting.list()).map((i) => i.title)).toEqual(['due', 'later', 'undated']);
  });

  it('clearing both fields takes an issue off the list', async () => {
    const mock = createMockApi();
    const issue = await mock.issues.create({ title: 'ask Sam' });
    await mock.issues.update(issue.id, { waiting_on: 'Sam', follow_up: '2999-01-01' });
    expect((await mock.issues.get(issue.id)).follow_up).toBe('2999-01-01');
    await mock.issues.update(issue.id, { waiting_on: null, follow_up: null });
    expect((await mock.waiting.list()).map((i) => i.id)).not.toContain(issue.id);
  });
});
