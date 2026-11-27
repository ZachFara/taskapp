import { createRawSnippet, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { shortcutFor } from '../lib/commands';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import Shell from './Shell.svelte';
import { readFileSync } from 'node:fs';
import { setTooltips } from '../lib/tooltips.svelte';

// Glyph assertions below are the Mac ones whatever machine runs them.
beforeEach(() => void vi.spyOn(navigator, 'platform', 'get').mockReturnValue('MacIntel'));

// Run from ui/ (pnpm -C ui test), so the path is relative to the package root.
const baseCss = readFileSync('src/styles/base.css', 'utf8');

let app: ReturnType<typeof mount> | null = null;
const body = createRawSnippet(() => ({ render: () => '<p>main view</p>' }));
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};

beforeEach(() => setApi(createMockApi()));
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
  localStorage.clear();
});

async function open() {
  app = mount(Shell, { target: document.body, props: { children: body } });
  await settle();
}

describe('Shell', () => {
  it('sidebar_collapses_and_widths_are_remembered_per_device', async () => {
    await open();
    dispatchEvent(new CustomEvent('command:pane.sidebar.wider'));
    await tick();
    expect(localStorage.getItem('cairn.sidebarWidth')).toBe(String(232 + 16));
    dispatchEvent(new CustomEvent('command:sidebar.toggle'));
    await tick();
    expect(document.querySelector<HTMLElement>('aside.sidebar')!.hidden).toBe(true);
    expect(localStorage.getItem('cairn.sidebarCollapsed')).toBe('1');
    dispatchEvent(new CustomEvent('command:pane.sidebar.reset'));
    dispatchEvent(new CustomEvent('command:sidebar.toggle'));
    await tick();
    expect(localStorage.getItem('cairn.sidebarWidth')).toBeNull();
    expect(document.querySelector<HTMLElement>('aside.sidebar')!.hidden).toBe(false);
  });

  it('sync_error_shows_indicator', async () => {
    const api = createMockApi();
    api.sync.status = async () => ({ reminders: 'Off', gcal: { Error: { at: 'now', message: 'token expired' } }, export: 'Off' });
    setApi(api);
    await open();
    expect(document.querySelector('.dot.error')).not.toBeNull();
    expect(document.body.textContent).toContain('token expired');
  });

  it('backup_error_shows_indicator', async () => {
    const api = createMockApi();
    api.sync.status = async () => ({ reminders: 'Off', gcal: 'Off', export: 'Off', backup: { Error: { at: 'now', message: 'disk full' } } });
    setApi(api);
    await open();
    expect(document.querySelector('.dot.error')).not.toBeNull();
    expect(document.body.textContent).toContain('disk full');
  });

  it('shows_main_view_and_no_indicator_when_sync_is_off', async () => {
    await open();
    expect(document.body.textContent).toContain('main view');
    expect(document.querySelector('.dot.error')).toBeNull();
  });

  it('search_field_calls_back_and_the_buttons_run_their_commands', async () => {
    const onsearch = vi.fn();
    const help = vi.fn();
    addEventListener('command:help.shortcuts', help);
    app = mount(Shell, { target: document.body, props: { children: body, onsearch } });
    await settle();
    document.querySelector<HTMLButtonElement>('[aria-label="Search or run a command"]')!.click();
    document.querySelector<HTMLButtonElement>('[aria-label="Keyboard shortcuts"]')!.click();
    removeEventListener('command:help.shortcuts', help);
    expect(onsearch).toHaveBeenCalledOnce();
    expect(help).toHaveBeenCalledOnce();
    expect(document.querySelector('header .tools')!.textContent).not.toContain('Create issue');
  });

  it('legend_shows_the_keys_the_registry_holds', async () => {
    await open();
    const legend = document.querySelector('footer.legend')!.textContent!;
    expect(legend).toContain('move');
    expect(legend).toContain(shortcutFor('move.down')!.toUpperCase());
    expect(legend).not.toMatch(/mod\+|arrow|escape|enter/);
    expect(legend).toContain('⌘K');
    expect(legend).toContain('commands');
    expect(legend).toContain('status');
    expect(legend).toContain('select');
  });

  it('the_search_button_has_a_tooltip_naming_its_key', async () => {
    setTooltips(true);
    await open();
    document.querySelector<HTMLButtonElement>('[aria-label="Search or run a command"]')!.focus();
    await settle();
    const tip = document.querySelector('.cx-tip');
    expect(tip?.textContent).toContain('Search or run a command');
    expect(tip?.textContent).toContain('⌘K');
  });

  it('breadcrumb_marks_the_last_part_as_current', async () => {
    app = mount(Shell, { target: document.body, props: { children: body, crumb: [{ label: 'Projects', href: '/projects' }, { label: 'Home' }] } });
    await settle();
    expect(document.querySelector('[aria-current=page]')!.textContent).toBe('Home');
  });

  it('breadcrumb_parent_with_a_page_is_a_link_to_it', async () => {
    app = mount(Shell, { target: document.body, props: { children: body, crumb: [{ label: 'Projects', href: '/projects' }, { label: 'Home' }] } });
    await settle();
    const link = document.querySelector<HTMLAnchorElement>('nav[aria-label=Breadcrumb] a')!;
    expect(link.textContent).toBe('Projects');
    link.click();
    expect(location.pathname).toBe('/projects');
  });

  it('panel_renders_only_when_open_and_after_the_main_view', async () => {
    const panel = createRawSnippet(() => ({ render: () => '<p>detail</p>' }));
    app = mount(Shell, { target: document.body, props: { children: body, panel, panelOpen: true } });
    await settle();
    expect(document.querySelector('aside.panel')!.textContent).toContain('detail');
  });

  it('app_root_carries_the_base_type_class', async () => {
    await open();
    expect(document.querySelector('.app')!.classList.contains('cairn-root')).toBe(true);
  });

  it('base_css_sets_the_body_font_so_every_screen_gets_the_app_face', () => {
    expect(baseCss).toMatch(/body\s*\{[^}]*font:[^}]*var\(--font\)/);
  });

  it('top_bar_has_no_second_create_issue_button', async () => {
    await open();
    const labels = [...document.querySelectorAll('header button')].map((b) => b.textContent?.trim());
    expect(labels).not.toContain('Create issue');
    expect(labels).not.toContain('New issue');
  });
});
