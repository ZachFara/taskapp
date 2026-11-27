import { expect, it } from 'vitest';
import { listCommands, type Ctx } from './index';
import { theme } from '../theme.svelte';

it('Change theme… offers a page with every look and applies the chosen one', async () => {
  const ctx = { route: '/', layout: 'list', selection: [], focusedIssueId: null, openIssueId: null, api: {}, navigate: () => {} } as unknown as Ctx;
  const c = listCommands(ctx, { all: true }).find((x) => x.id === 'theme.skin')!;
  const items = await c.page!(ctx).items;
  expect(items.map((i) => i.id)).toContain('scholastic');
  await items.find((i) => i.id === 'cupertino')!.run();
  expect(theme.skin).toBe('cupertino');
  document.documentElement.removeAttribute('data-skin');
});
