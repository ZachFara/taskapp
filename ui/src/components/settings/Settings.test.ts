import { afterEach, describe, expect, it, vi } from 'vitest';
import { features } from '../../lib/features.svelte';
import '../../lib/commands';
import { labelFor, shortcutFor } from '../../lib/commands';
import Settings from './Settings.svelte';
import { mountSettings } from './testing';

describe('Settings', () => {
  afterEach(() => Object.assign(features, { cycles: true, reminders: true, gcal: true }));

  it('hides the settings pages of a feature that is switched off', async () => {
    Object.assign(features, { cycles: false, reminders: false, gcal: false });
    history.pushState(null, '', '/settings/general');
    const { target } = await mountSettings(Settings);
    const names = [...target.querySelectorAll('nav a')].map((a) => a.textContent?.trim());
    expect(names).not.toContain('Cycles');
    expect(names).not.toContain('Reminders');
    expect(names).not.toContain('Calendar');
    expect(names).toContain('Features');
  });

  it('routes each section and marks the current one', async () => {
    history.pushState(null, '', '/settings/export');
    const { target } = await mountSettings(Settings);
    expect(target.querySelector('[aria-current="page"]')?.textContent).toBe('Export');
    expect(target.querySelector('h1')?.textContent).toBe('Settings / Export');
    target.querySelector<HTMLAnchorElement>('nav a[href="/settings/backups"]')!.click();
    await new Promise((r) => setTimeout(r, 0));
    expect(location.pathname).toBe('/settings/backups');
    expect(target.querySelector('h1')?.textContent).toBe('Settings / Backups');
  });

  it('each section names itself once, in the page header', async () => {
    for (const id of ['general', 'statuses', 'labels', 'projects', 'cycles', 'properties', 'relations', 'templates', 'import', 'reminders', 'calendar', 'export', 'backups']) {
      history.pushState(null, '', `/settings/${id}`);
      const { target } = await mountSettings(Settings);
      const label = target.querySelector('nav a[aria-current="page"]')!.textContent!.trim();
      expect(target.querySelectorAll('h1').length, id).toBe(1);
      expect(target.querySelector('h1')?.textContent, id).toBe(`Settings / ${label}`);
      expect([...target.querySelectorAll('h2')].map((h) => h.textContent?.trim()), id).not.toContain(label);
      target.remove();
    }
  });

  it('lists every section in the left nav, shortcuts included', async () => {
    history.pushState(null, '', '/settings/general');
    const { target } = await mountSettings(Settings);
    const names = [...target.querySelectorAll('nav a')].map((a) => a.textContent?.trim());
    expect(names).toEqual([
      'General', 'Features', 'Statuses', 'Project statuses', 'Labels', 'Projects', 'Cycles', 'Fields', 'Properties', 'Relations', 'Templates', 'Import',
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
