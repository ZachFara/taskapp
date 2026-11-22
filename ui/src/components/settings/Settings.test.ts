import { describe, expect, it, vi } from 'vitest';
import '../../lib/commands';
import { labelFor, shortcutFor } from '../../lib/commands';
import Settings from './Settings.svelte';
import { mountSettings } from './testing';

describe('Settings', () => {
  it('routes each section and marks the current one', async () => {
    history.pushState(null, '', '/settings/export');
    const { target } = await mountSettings(Settings);
    expect(target.querySelector('[aria-current="page"]')?.textContent).toBe('Export');
    expect(target.querySelector('h2')?.textContent).toBe('Export');
    target.querySelector<HTMLAnchorElement>('nav a[href="/settings/backups"]')!.click();
    await new Promise((r) => setTimeout(r, 0));
    expect(location.pathname).toBe('/settings/backups');
    expect(target.querySelector('h2')?.textContent).toBe('Backups');
  });

  it('lists every section in the left nav, shortcuts included', async () => {
    history.pushState(null, '', '/settings/general');
    const { target } = await mountSettings(Settings);
    const names = [...target.querySelectorAll('nav a')].map((a) => a.textContent?.trim());
    expect(names).toEqual([
      'General', 'Statuses', 'Labels', 'Projects', 'Cycles', 'Properties', 'Templates', 'Import',
      'Reminders', 'Calendar', 'Export', 'Backups', 'Keyboard shortcuts',
    ]);
    expect(target.textContent).toContain('Keyboard shortcuts');
  });

  it('a section with no screen yet shows a designed empty state, not a crash', async () => {
    history.pushState(null, '', '/settings/nope');
    const { target } = await mountSettings(Settings);
    expect(target.querySelector('h2')?.textContent).toMatch(/no settings page/i);
  });

  it('import and templates show their real screens, never the empty state', async () => {
    history.pushState(null, '', '/settings/import');
    const imp = await mountSettings(Settings);
    expect(imp.target.textContent).not.toMatch(/no settings page/i);
    expect(imp.target.querySelector('input[type="file"], [data-dropzone], .drop')).not.toBeNull();
    imp.target.remove();
    history.pushState(null, '', '/settings/templates');
    const tpl = await mountSettings(Settings);
    expect(tpl.target.textContent).toContain('New template');
    expect(tpl.target.textContent).not.toMatch(/no settings page/i);
  });

  it('the settings nav sits on the sheet, not on the dark frame', async () => {
    history.pushState(null, '', '/settings/general');
    const { target } = await mountSettings(Settings);
    expect(target.querySelector('nav')!.hasAttribute('data-on-frame')).toBe(false);
  });

  it('keyboard shortcuts entry asks for the cheat sheet command', async () => {
    history.pushState(null, '', '/settings/general');
    const { target } = await mountSettings(Settings);
    const seen = vi.fn();
    addEventListener('command:help.shortcuts', seen);
    target.querySelector<HTMLAnchorElement>('nav a[href="#shortcuts"]')!.click();
    removeEventListener('command:help.shortcuts', seen);
    expect(seen).toHaveBeenCalledOnce();
  });

  it('g s opens settings (the nav command owns the key)', async () => {
    expect(shortcutFor('nav.settings')).toBe('g s');
    expect(labelFor('nav.settings')).toBe('Go to Settings');
  });
});
