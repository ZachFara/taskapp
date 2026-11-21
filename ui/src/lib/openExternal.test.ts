import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const openUrl = vi.fn();
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: (...args: unknown[]) => openUrl(...args) }));

import { openExternal } from './openExternal';

describe('openExternal', () => {
  beforeEach(() => {
    vi.spyOn(window, 'open').mockImplementation(() => null);
    openUrl.mockReset();
  });
  afterEach(() => {
    vi.restoreAllMocks();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it.each(['javascript:alert(1)', 'file:///etc/passwd', 'data:text/html,hi', 'vbscript:x', ''])('refuses %s', async (url) => {
    expect(await openExternal(url)).toBe(false);
    expect(window.open).not.toHaveBeenCalled();
    expect(openUrl).not.toHaveBeenCalled();
  });

  it('uses window.open outside Tauri', async () => {
    expect(await openExternal('https://example.com/x')).toBe(true);
    expect(window.open).toHaveBeenCalledWith('https://example.com/x', '_blank', 'noopener');
    expect(openUrl).not.toHaveBeenCalled();
  });

  it('hands obsidian and http links to window.open outside Tauri', async () => {
    await openExternal('obsidian://open?vault=v');
    await openExternal('http://localhost:8080');
    expect(window.open).toHaveBeenCalledWith('obsidian://open?vault=v', '_blank', 'noopener');
    expect(window.open).toHaveBeenCalledWith('http://localhost:8080', '_blank', 'noopener');
  });

  it('calls the opener plugin inside Tauri', async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    openUrl.mockResolvedValue(undefined);
    expect(await openExternal('obsidian://open?vault=v')).toBe(true);
    expect(openUrl).toHaveBeenCalledWith('obsidian://open?vault=v');
    expect(window.open).not.toHaveBeenCalled();
  });

  it('refuses in Tauri too, before the plugin is reached', async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(await openExternal('file:///etc/passwd')).toBe(false);
    expect(openUrl).not.toHaveBeenCalled();
  });
});
