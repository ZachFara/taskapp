import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, httpApi, setApi } from './api';
import { createMockApi } from './api.mock';

const wrote = vi.fn();
window.addEventListener('history:wrote', wrote);
afterEach(() => {
  wrote.mockClear();
  vi.unstubAllGlobals();
});

describe('history:wrote', () => {
  it('fires_after_a_mock_write_and_not_after_a_read', async () => {
    setApi(createMockApi());
    await api.issues.create({ title: 'x' });
    expect(wrote).toHaveBeenCalledTimes(1);
    await api.issues.get(1).catch(() => {});
    await api.views.list();
    await api.history.undoStatus();
    expect(wrote).toHaveBeenCalledTimes(1);
  });

  it('does_not_fire_when_the_write_fails', async () => {
    setApi(createMockApi());
    await api.issues.update(999999, {}).catch(() => {});
    expect(wrote).not.toHaveBeenCalled();
  });

  it('does_not_fire_for_undo_and_redo_which_show_their_own_toast', async () => {
    setApi(createMockApi());
    await api.history.undo().catch(() => {});
    await api.history.redo().catch(() => {});
    expect(wrote).not.toHaveBeenCalled();
  });

  it('fires_once_when_the_same_api_is_installed_twice', async () => {
    const a = createMockApi();
    setApi(a);
    setApi(a);
    await api.issues.create({ title: 'x' });
    expect(wrote).toHaveBeenCalledTimes(1);
  });

  it('fires_on_the_http_path_for_writes_only', async () => {
    vi.stubGlobal('fetch', async () => ({ json: async () => ({ ok: null }) }));
    const h = httpApi('http://x');
    await h.views.list();
    expect(wrote).not.toHaveBeenCalled();
    await h.pins.unpin(1);
    expect(wrote).toHaveBeenCalledTimes(1);
  });
});
