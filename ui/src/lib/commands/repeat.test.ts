import { describe, expect, it } from 'vitest';
import { createMockApi } from '../api.mock';
import { setApi } from '../api';
import { commandById, type CommandPage, type Ctx } from './index';

const setup = async () => {
  const api = createMockApi();
  setApi(api);
  const issue = await api.issues.create({ title: 'Water plants', due: { Date: '2999-01-02' } });
  const ctx = { api, route: '/my', selection: [], focusedIssueId: issue.id, openIssueId: null, navigate: () => {} } as unknown as Ctx;
  const page = commandById('issue.repeat')!.page!(ctx) as CommandPage;
  return { api, issue, page, items: (await page.items) as { label: string; run: () => Promise<void> }[] };
};

describe('issue.repeat', () => {
  it('offers presets that follow the due date, sets one, and stops repeating', async () => {
    const { api, issue, items } = await setup();
    expect(items.map((i) => i.label)).toContain('Every week on Wednesday'); // 2999-01-02 is a Wednesday
    expect(items.map((i) => i.label)).toContain('Every month on day 2');
    await items.find((i) => i.label === 'Every week on Wednesday')!.run();
    expect((await api.issues.get(issue.id)).repeat).toBe('every 1 week on wed from due');
    await items.find((i) => i.label === 'Stop repeating')!.run();
    expect((await api.issues.get(issue.id)).repeat).toBeNull();
  });
  it('takes typed text and ignores text that is not a rule', async () => {
    const { api, issue, page } = await setup();
    expect(page.parse!('sometimes')).toBeNull();
    await page.parse!('every 2 weeks from completion')!.run();
    expect((await api.issues.get(issue.id)).repeat).toBe('every 2 week from done');
  });
  it('skip moves the due date of a repeating issue', async () => {
    const { api, issue, items } = await setup();
    await items.find((i) => i.label === 'Every day')!.run();
    const ctx = { api, route: '/my', selection: [], focusedIssueId: issue.id, openIssueId: null, navigate: () => {} } as unknown as Ctx;
    await commandById('issue.repeat.skip')!.run!(ctx);
    expect((await api.issues.get(issue.id)).due).toEqual({ Date: '2999-01-03' });
  });
});
