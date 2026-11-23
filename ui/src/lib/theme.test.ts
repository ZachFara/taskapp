import { afterEach, describe, expect, it, vi } from 'vitest';
import { initTheme, setSkin, setTheme, theme } from './theme.svelte';

const root = () => document.documentElement;
const flush = () => new Promise((r) => setTimeout(r, 0));

afterEach(() => {
  root().removeAttribute('data-theme');
  root().removeAttribute('data-skin');
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

  it('a skin is saved, cached and applied; the first skin removes the attribute', () => {
    const settings = { get: vi.fn(async () => null), set: vi.fn(async () => {}) };
    initTheme({ settings } as never);
    setSkin('scholastic');
    expect(settings.set).toHaveBeenCalledWith('skin', 'scholastic');
    expect(localStorage.getItem('cairn.skin')).toBe('scholastic');
    expect(root().dataset.skin).toBe('scholastic');
    setSkin('survey', false);
    expect(root().hasAttribute('data-skin')).toBe(false);
  });

  it('the saved skin wins over the cache', async () => {
    const settings = { get: vi.fn(async (k: string) => (k === 'skin' ? 'jira' : null)), set: vi.fn(async () => {}) };
    initTheme({ settings } as never);
    await flush();
    expect(root().dataset.skin).toBe('jira');
  });
});
