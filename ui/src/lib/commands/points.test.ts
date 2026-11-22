import { expect, it } from 'vitest';
import { api, setApi } from '../api';
import { createMockApi } from '../api.mock';
import { listCommands, type Ctx } from './index';

const ctx = { route: '/view/1', layout: 'list', selection: [1], focusedIssueId: 1, openIssueId: null, navigate: () => {} } as unknown as Ctx;
const offered = () => listCommands(ctx).some((c) => c.id === 'issue.points');
const settle = () => new Promise((r) => setTimeout(r, 20));

it('Set points is offered only when estimation is points or both', async () => {
  setApi(createMockApi());
  for (const [value, want] of [['none', false], ['time', false], ['points', true], ['both', true]] as const) {
    await api.settings.set('estimation', value); // a settings write announces itself like any write
    await settle();
    expect(offered(), value).toBe(want);
  }
});
