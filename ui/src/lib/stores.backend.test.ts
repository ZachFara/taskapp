import { afterEach, expect, it, vi } from 'vitest';

const handlers: Record<string, (e: { payload: unknown }) => void> = {};
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, fn: (e: { payload: unknown }) => void) => {
    handlers[name] = fn;
  },
}));
vi.mock('./api', () => ({
  api: {
    views: { list: vi.fn(async () => []), query: vi.fn(async () => []) },
    statuses: { list: vi.fn(async () => []) },
    projects: { list: vi.fn(async () => []) },
    properties: { listDefs: vi.fn(async () => []) },
  },
}));

afterEach(() => vi.restoreAllMocks());

it('db-changed reloads the stores and open-issue navigates', async () => {
  const { listenToBackend } = await import('./stores.svelte');
  const { api } = await import('./api');
  await listenToBackend();
  handlers['db-changed']({ payload: null });
  await vi.waitFor(() => expect(api.views.list).toHaveBeenCalled());
  handlers['open-issue']({ payload: 'ZF-7' });
  expect(location.pathname).toBe('/issue/ZF-7');
});
