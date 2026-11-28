import { describe, expect, it } from 'vitest';
import { createMockApi } from '../api.mock';
import { setApi } from '../api';
import { commandById, type CommandPage, type Ctx } from './index';

describe('issue.depth', () => {
  it('sets deep and shallow, and clears the depth, from its command page', async () => {
    const api = createMockApi();
    setApi(api);
    const issue = await api.issues.create({ title: 'Think it through' });
    const ctx = { api, route: '/my', selection: [], focusedIssueId: issue.id, openIssueId: null, navigate: () => {} } as unknown as Ctx;
    const page = commandById('issue.depth')!.page!(ctx) as CommandPage;
    const items = (await page.items) as { label: string; run: () => Promise<void> }[];
    const pick = (label: string) => items.find((i) => i.label === label)!.run();
    await pick('Deep work');
    expect((await api.issues.get(issue.id)).depth).toBe('deep');
    await pick('Shallow work');
    expect((await api.issues.get(issue.id)).depth).toBe('shallow');
    await pick('No depth');
    expect((await api.issues.get(issue.id)).depth).toBeNull();
  });
});
