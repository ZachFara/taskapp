import { afterEach, describe, expect, it, vi } from 'vitest';
import { initTheme, setTheme, theme } from './theme.svelte';

const root = () => document.documentElement;
const flush = () => new Promise((r) => setTimeout(r, 0));

afterEach(() => {
  root().removeAttribute('data-theme');
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('theme store', () => {
  it('the saved setting wins over the startup cache and is applied', async () => {
    localStorage.setItem('cairn.theme', 'light');
    const settings = { get: vi.fn(async () => 'dark'), set: vi.fn(async () => {}) };
    initTheme({ settings } as never);
    await flush();
    expect(theme.mode).toBe('dark');
    expect(root().dataset.theme).toBe('dark');
    expect(settings.set).not.toHaveBeenCalled();
  });

  it('a choice is written through the settings api and mirrored to the cache', async () => {
    const settings = { get: vi.fn(async () => null), set: vi.fn(async () => {}) };
    initTheme({ settings } as never);
    setTheme('light');
    expect(settings.set).toHaveBeenCalledWith('theme', 'light');
    expect(localStorage.getItem('cairn.theme')).toBe('light');
    expect(root().dataset.theme).toBe('light');
  });

  it('system removes the override so the OS setting applies', () => {
    setTheme('dark', false);
    setTheme('system', false);
    expect(root().hasAttribute('data-theme')).toBe(false);
  });
});
