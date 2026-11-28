import { describe, expect, it } from 'vitest';
import { createMockApi } from '../api.mock';
import { setApi } from '../api';
import { featureOfCommand } from '../features.svelte';
import { commandById, type CommandPage, type Ctx } from './index';

describe('issue.defer', () => {
  it('presets and typed days set hidden_until, a past day is not offered, and Bring it back now clears it', async () => {
    const api = createMockApi();
    setApi(api);
    const issue = await api.issues.create({ title: 'Check car sticker' });
    const ctx = { api, route: '/my', selection: [], focusedIssueId: issue.id, openIssueId: null, navigate: () => {} } as unknown as Ctx;
    const page = commandById('issue.defer')!.page!(ctx) as CommandPage;
    const items = (await page.items) as { label: string; run: () => Promise<void> }[];
    expect(items.map((i) => i.label)).toEqual(['Tomorrow', 'In a week', 'In a month', 'Bring it back now']);
    await items.find((i) => i.label === 'In a week')!.run();
    expect((await api.issues.get(issue.id)).hidden_until).not.toBeNull();
    expect(page.parse!('2000-01-01')).toBeNull();
    await page.parse!('2999-03-04')!.run();
    expect((await api.issues.get(issue.id)).hidden_until).toBe('2999-03-04');
    await items.find((i) => i.label === 'Bring it back now')!.run();
    expect((await api.issues.get(issue.id)).hidden_until).toBeNull();
  });

  it('belongs to the process_inbox feature, like the Tickler page', () => {
    expect(featureOfCommand('issue.defer')).toBe('process_inbox');
    expect(featureOfCommand('nav.tickler')).toBe('process_inbox');
  });
});
