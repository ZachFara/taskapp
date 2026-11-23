import { afterEach, expect, it, vi } from 'vitest';

afterEach(() => vi.unstubAllEnvs());

// The gallery is a developer tool: a shipped build offers no command, key or menu row for it.
it('a production build has no Go to Design gallery command', async () => {
  vi.stubEnv('DEV', false);
  vi.resetModules();
  const reg = await import('./registry');
  await import('./defs/nav/design');
  expect(reg.listCommands({ route: '/', selection: [], focusedIssueId: null, openIssueId: null } as never, { all: true }).map((c) => c.id)).not.toContain('nav.design');
});
