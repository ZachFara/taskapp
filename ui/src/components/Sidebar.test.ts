import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import Sidebar from './Sidebar.svelte';

const def = { filter: null, sorts: [], group_by: null, layout: 'List' as const, columns: [], include_archived: false };
const seed = { ...fixtures, views: ['Alpha', 'Beta', 'Gamma'].map((name, i) => ({ id: i + 1, name, def, position: i })) };
const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const names = (sel: string) => [...document.querySelectorAll(`${sel} a`)].map((a) => a.textContent);
const fire = (el: Element, type: string) => el.dispatchEvent(new Event(type, { bubbles: true, cancelable: true }));
const row = (text: string) => [...document.querySelectorAll('.row')].find((r) => r.textContent?.includes(text))!;
const dragOnto = (from: string, to: string) => {
  fire(row(from), 'dragstart');
  fire(row(to), 'drop');
};

let mock: Api;
let app: ReturnType<typeof mount>;
async function open(api: Api = createMockApi(seed)) {
  mock = api;
  setApi(api);
  app = mount(Sidebar, { target: document.body });
  await settle();
}
beforeEach(() => vi.stubGlobal('open', vi.fn()));
afterEach(() => {
  unmount(app);
  vi.unstubAllGlobals();
});

describe('Sidebar', () => {
  it('pin_toggle_on_view_row', async () => {
    await open();
    const btn = () => document.querySelector<HTMLButtonElement>('[aria-label="Pin Alpha"], [aria-label="Unpin Alpha"]')!;
    btn().click();
    await settle();
    expect(names('[aria-label=Pinned]')).toEqual(['Alpha']);
    expect(btn().getAttribute('aria-label')).toBe('Unpin Alpha');
    btn().click();
    await settle();
    expect(names('[aria-label=Pinned]')).toEqual([]);
  });

  it('pinned_section_lists_and_reorders', async () => {
    const api = createMockApi(seed);
    await api.pins.pin('View', '1', 'One');
    await api.pins.pin('Project', 'website', 'Home');
    await open(api);
    expect(names('[aria-label=Pinned]')).toEqual(['One', 'Home']);
    dragOnto('Home', 'One');
    await settle();
    expect(names('[aria-label=Pinned]')).toEqual(['Home', 'One']);
    expect((await api.pins.listResolved()).map((r) => r.label)).toEqual(['Home', 'One']);
  });

  it('unpin_on_hover_button', async () => {
    const api = createMockApi(seed);
    await api.pins.pin('View', '1', 'One');
    await open(api);
    document.querySelector<HTMLButtonElement>('[aria-label="Unpin One"]')!.click();
    await settle();
    expect(await api.pins.listResolved()).toEqual([]);
  });

  it('reorder_views', async () => {
    await open();
    expect(names('[aria-label=Views]')).toEqual(['Alpha', 'Beta', 'Gamma']);
    dragOnto('Gamma', 'Alpha');
    await settle();
    expect(names('[aria-label=Views]')).toEqual(['Gamma', 'Alpha', 'Beta']);
    expect((await mock.views.list()).map((v) => v.id)).toEqual([3, 1, 2]);
  });

  it('reorder_with_failed_api_restores_old_order', async () => {
    const api = createMockApi(seed);
    await api.pins.pin('View', '1', 'One');
    await api.pins.pin('Project', 'website', 'Home');
    api.pins.reorder = async () => {
      throw { kind: 'Io', message: 'disk full' };
    };
    await open(api);
    dragOnto('Home', 'One');
    await settle();
    expect(names('[aria-label=Pinned]')).toEqual(['One', 'Home']);
    expect(document.querySelector('[role=alert]')?.textContent).toBe('disk full');
  });

  it('external_pin_opens_externally', async () => {
    // The backend refuses a javascript: pin, so it can only arrive as old data; seed it directly.
    const url = (id: number, target: string, title: string) => ({ id, kind: 'Url' as const, target, title, position: id });
    const api = createMockApi({ ...seed, pins: [url(1, 'https://example.com/x', 'Site'), url(2, 'javascript:alert(1)', 'Bad')] });
    await open(api);
    const pushed = vi.spyOn(history, 'pushState');
    row('Site').querySelector('a')!.click();
    expect(window.open).toHaveBeenCalledWith('https://example.com/x', '_blank', 'noopener');
    row('Bad').querySelector('a')!.click();
    expect(window.open).toHaveBeenCalledTimes(1);
    expect(pushed).not.toHaveBeenCalled();
  });

  it('active_route_gets_the_current_page_marker', async () => {
    const api = createMockApi(seed);
    setApi(api);
    app = mount(Sidebar, { target: document.body, props: { route: '/view/2' } });
    await settle();
    const on = [...document.querySelectorAll('[aria-current=page]')].map((a) => a.textContent);
    expect(on).toEqual(['Beta']);
  });

  it('sections_collapse_and_show_counts', async () => {
    await open();
    const head = document.querySelector<HTMLButtonElement>('[aria-label=Views] .head')!;
    expect(head.textContent).toContain('3');
    expect(head.getAttribute('aria-expanded')).toBe('true');
    head.click();
    await settle();
    expect(head.getAttribute('aria-expanded')).toBe('false');
    expect(names('[aria-label=Views]')).toEqual([]);
  });

  it('top_items_and_settings_navigate', async () => {
    await open();
    const link = (t: string) => [...document.querySelectorAll('a')].find((a) => a.textContent === t)!;
    link('My issues').click();
    expect(location.pathname).toBe('/my');
    link('Settings').click();
    expect(location.pathname).toBe('/settings/general');
  });
});

describe('Sidebar icons', () => {
  it('a view without its own icon shows the icon of its layout', async () => {
    const board = { ...def, layout: 'Board' as const };
    await open(createMockApi({ ...seed, views: [{ id: 1, name: 'Kanban', def: board, position: 0, icon: null, color: null }] }));
    expect(row('Kanban').querySelector('[data-icon]')?.getAttribute('data-icon')).toBe('board');
  });

  it('a project shows the project icon, not the hollow circle', async () => {
    await open();
    const first = fixtures.projects[0].name;
    expect(row(first).querySelector('[data-icon]')?.getAttribute('data-icon')).toBe('project');
  });

  it('shows a sparkline on each project row only when the setting is on', async () => {
    await open();
    expect(document.querySelector('[aria-label="Projects"] path.spark')).toBeNull();
    unmount(app);
    const on = createMockApi(seed);
    await on.settings.set('sidebar_sparklines', true);
    await open(on);
    expect(document.querySelectorAll('[aria-label="Projects"] path.spark')).toHaveLength(fixtures.projects.length);
  });
});

describe('Sidebar without an inbox', () => {
  it('lists no Inbox link and keeps My issues', async () => {
    await open();
    const links = [...document.querySelectorAll('nav a')].map((a) => a.textContent?.trim());
    expect(links).not.toContain('Inbox');
    expect(links).toContain('My issues');
  });
});

describe('Sidebar tooltips', () => {
  const tip = async (el: Element | null) => {
    (el as HTMLElement).focus();
    await settle();
    return document.querySelector('.cx-tip')?.textContent ?? '';
  };

  it('a pin button names its action', async () => {
    await open();
    expect(await tip(document.querySelector('[aria-label="Pin Alpha"]'))).toContain('Pin Alpha');
  });

  it('a section link shows the go-to key', async () => {
    await open();
    const t = await tip([...document.querySelectorAll('nav a')].find((a) => a.textContent?.includes('My issues'))!);
    expect(t).toContain('My issues');
    expect(t).toMatch(/G\s*M|GM/);
  });
});
