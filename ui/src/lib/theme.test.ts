import { afterEach, describe, expect, it } from 'vitest';
import { cycleTheme, setTheme, theme } from './theme.svelte';

const attr = () => document.documentElement.getAttribute('data-theme');
afterEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
});

describe('theme', () => {
  it('sets_data_theme_for_light_and_dark', () => {
    setTheme('dark');
    expect(attr()).toBe('dark');
    setTheme('light');
    expect(attr()).toBe('light');
  });

  it('system_removes_the_attribute_so_the_media_query_decides', () => {
    setTheme('dark');
    setTheme('system');
    expect(attr()).toBeNull();
  });

  it('cycles_light_dark_system_and_remembers_the_choice', () => {
    setTheme('light');
    cycleTheme();
    expect(theme.mode).toBe('dark');
    cycleTheme();
    expect(theme.mode).toBe('system');
    expect(localStorage.getItem('cairn.theme')).toBe('system');
  });

  it('works_when_storage_throws', () => {
    const real = Storage.prototype.setItem;
    Storage.prototype.setItem = () => {
      throw new Error('blocked');
    };
    try {
      expect(() => setTheme('dark')).not.toThrow();
      expect(attr()).toBe('dark');
    } finally {
      Storage.prototype.setItem = real;
    }
  });
});
