import { describe, expect, it } from 'vitest';
import { createMockApi } from '../api.mock';
import { setApi } from '../api';
import { featureOfCommand } from '../features.svelte';
import { commandById, type CommandPage, type Ctx } from './index';

const setup = async () => {
  const api = createMockApi();
  setApi(api);
  const issue = await api.issues.create({ title: 'Send the contract' });
  const ctx = { api, route: '/my', selection: [], focusedIssueId: issue.id, openIssueId: null, navigate: () => {} } as unknown as Ctx;
  return { api, issue, ctx };
};

describe('issue.wait and issue.followUp', () => {
  it('a typed name sets who the issue waits on, and Stop waiting clears it and the follow-up day', async () => {
    const { api, issue, ctx } = await setup();
    const page = commandById('issue.wait')!.page!(ctx) as CommandPage;
    expect(page.parse!('   ')).toBeNull();
    await page.parse!('Sam')!.run();
    expect((await api.issues.get(issue.id)).waiting_on).toBe('Sam');
    await api.issues.update(issue.id, { follow_up: '2999-01-01' });
    const items = (await page.items) as { label: string; run: () => Promise<void> }[];
    await items.find((i) => i.label === 'Stop waiting')!.run();
    expect((await api.issues.get(issue.id)).waiting_on).toBeNull();
    expect((await api.issues.get(issue.id)).follow_up).toBeNull();
  });

  it('the follow-up page takes presets and typed days', async () => {
    const { api, issue, ctx } = await setup();
    const page = commandById('issue.followUp')!.page!(ctx) as CommandPage;
    const items = (await page.items) as { label: string; run: () => Promise<void> }[];
    expect(items.map((i) => i.label)).toEqual(['Tomorrow', 'In a week', 'No follow-up day']);
    await items.find((i) => i.label === 'In a week')!.run();
    expect((await api.issues.get(issue.id)).follow_up).not.toBeNull();
    expect(page.parse!('nonsense words here')).toBeNull();
    await page.parse!('2999-03-04')!.run();
    expect((await api.issues.get(issue.id)).follow_up).toBe('2999-03-04');
  });

  it('belongs to the waiting feature', () => {
    expect(featureOfCommand('issue.wait')).toBe('waiting');
    expect(featureOfCommand('issue.followUp')).toBe('waiting');
    expect(featureOfCommand('nav.waiting')).toBe('waiting');
  });
});
