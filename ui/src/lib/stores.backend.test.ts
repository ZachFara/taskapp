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
    folders: { list: vi.fn(async () => []), items: vi.fn(async () => []) },
    project: { statusesList: vi.fn(async () => []) },
  },
}));

afterEach(() => vi.restoreAllMocks());

it('db-changed reloads the stores and open-issue navigates', async () => {
  const { listenToBackend } = await import('./stores.svelte');
  const { api } = await import('./api');
  const rowsChanged = vi.fn();
  window.addEventListener('rows:changed', rowsChanged);
  await listenToBackend();
  handlers['db-changed']({ payload: null });
  await vi.waitFor(() => expect(rowsChanged).toHaveBeenCalledOnce());
  expect(api.views.list).toHaveBeenCalled();
  window.removeEventListener('rows:changed', rowsChanged);
  handlers['open-issue']({ payload: 'ABC-7' });
  expect(location.pathname).toBe('/issue/ABC-7');
});

it('open-link navigates to the item a taskapp link names', async () => {
  const { listenToBackend } = await import('./stores.svelte');
  await listenToBackend();
  handlers['open-link']({ payload: 'taskapp://event/9' });
  await vi.waitFor(() => expect(location.pathname).toBe('/event/9'));
});

it('keeps the newest store refresh when an older query finishes later', async () => {
  const { refreshAll, issues } = await import('./stores.svelte');
  const { api } = await import('./api');
  let release!: (rows: never[]) => void;
  vi.mocked(api.views.query)
    .mockImplementationOnce(() => new Promise((resolve) => (release = resolve)))
    .mockResolvedValueOnce([{ id: 2, title: 'new' }] as never);
  const old = refreshAll();
  await vi.waitFor(() => expect(release).toBeDefined());
  await refreshAll();
  release([{ id: 1, title: 'old' }] as never);
  await old;
  expect(issues.map((i) => i.title)).toEqual(['new']);
});
