import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import { remove } from '../lib/rowActions';
import { refreshAll } from '../lib/stores.svelte';
import { rowUi } from '../lib/rowUi.svelte';
import { sidebarPrefs } from '../lib/sidebarPrefs.svelte';
import Sidebar from './Sidebar.svelte';
import { setTooltips } from '../lib/tooltips.svelte';

const def = { filter: null, sorts: [], group_by: null, layout: 'List' as const, columns: [], include_archived: false };
const seed = { ...fixtures, views: ['Alpha', 'Beta', 'Gamma'].map((name, i) => ({ id: i + 1, name, def, position: i })) };
const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const names = (sel: string) => [...document.querySelectorAll(`${sel} a`)].map((a) => a.textContent);
const menuItem = (re: RegExp) => {
  const m = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((e) => re.test(e.textContent?.trim() ?? ''));
  if (!m) throw new Error(`no menu item ${re}`);
  return m;
};
/** Opens a row's right-click menu and chooses the item. */
const pick = async (row: string, item: RegExp) => {
  // Sidebar rows show no button on hover; the menu opens on right-click.
  document.querySelector(`.rowactions[aria-label="${row}"]`)!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  flushSync();
  await vi.waitFor(() => menuItem(item).click());
  await settle();
};
const fire = (el: Element, type: string, init: MouseEventInit = {}) => el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, ...init }));
const row = (text: string) => [...document.querySelectorAll('.row')].find((r) => r.textContent?.includes(text))!;
/** A pointer drag of one row onto another, `frac` of the way down the target (rows have no size in jsdom, so 20px is given). */
const dragOnto = (from: string, to: string, frac = 0.5) => {
  const target = row(to);
  target.getBoundingClientRect = () => ({ top: 0, height: 20 }) as DOMRect;
  fire(row(from), 'pointerdown', { clientX: 0, clientY: 0 });
  fire(target, 'pointermove', { clientX: 20, clientY: 20 * frac });
  fire(target, 'pointerup', { clientX: 20, clientY: 20 * frac });
};
const folder = (id: number, name: string, position = 0, parent_id: number | null = null, section: 'view' | 'pin' | 'project' | 'cycle' = 'view') => ({ id, name, position, section, parent_id });

let mock: Api;
let app: ReturnType<typeof mount>;
async function open(api: Api = createMockApi(seed)) {
  mock = api;
  setApi(api);
  app = mount(Sidebar, { target: document.body });
  await settle();
}
beforeEach(() => {
  vi.stubGlobal('open', vi.fn());
  sidebarPrefs.showProjects = true; // the list most tests use; the tests of the switch turn it off
});
afterEach(() => {
  unmount(app);
  vi.unstubAllGlobals();
});

describe('Sidebar', () => {
  it('pin_toggle_on_view_row', async () => {
    await open();
    await pick('Alpha', /^Pin Alpha$/);
    expect(names('[aria-label=Pinned]')).toEqual(['Alpha']);
    await pick('Alpha', /^Unpin Alpha$/);
    expect(names('[aria-label=Pinned]')).toEqual([]);
  });

  it('pinned_section_lists_and_reorders', async () => {
    const api = createMockApi(seed);
    await api.pins.pin('View', '1', 'One');
    await api.pins.pin('Project', 'website', 'Home');
    await open(api);
    expect(names('[aria-label=Pinned]')).toEqual(['One', 'Home']);
    dragOnto('Home', 'One', 0.1);
    await settle();
    expect(names('[aria-label=Pinned]')).toEqual(['Home', 'One']);
    expect((await api.pins.listResolved()).map((r) => r.label)).toEqual(['Home', 'One']);
  });

  it('keeps unpin in the context menu instead of a hover button', async () => {
    const api = createMockApi(seed);
    await api.pins.pin('View', '1', 'One');
    await open(api);
    expect(document.querySelector('[aria-label="Unpin One"]')).toBeNull();
    await pick('One (pinned)', /^Unpin One$/);
    expect(await api.pins.listResolved()).toEqual([]);
    expect(names('[aria-label=Pinned]')).toEqual([]);
  });

  it('reorder_views', async () => {
    await open();
    expect(names('[aria-label=Views]')).toEqual(['Alpha', 'Beta', 'Gamma']);
    dragOnto('Gamma', 'Alpha', 0.1);
    await settle();
    expect(names('[aria-label=Views]')).toEqual(['Gamma', 'Alpha', 'Beta']);
    expect((await mock.views.list()).map((v) => v.id)).toEqual([3, 1, 2]);
  });

  it('a refused move leaves the old order', async () => {
    const api = createMockApi(seed);
    await api.pins.pin('View', '1', 'One');
    await api.pins.pin('Project', 'website', 'Home');
    api.folders.placeItem = async () => {
      throw { kind: 'Io', message: 'disk full' };
    };
    await open(api);
    dragOnto('Home', 'One', 0.1);
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

describe('Sidebar folders', () => {
  const withFolder = async () => {
    const api = createMockApi({ ...seed, viewFolders: [folder(1, 'Work')], views: seed.views.map((v) => ({ ...v, folder_id: v.id === 2 ? 1 : null, template_id: null })) });
    await open(api);
    return api;
  };
  const folderOf = async (api: Api, name: string) => (await api.views.list()).find((v) => v.name === name)!.folder_id;
  afterEach(() => localStorage.clear());

  it('shows a folder with its views indented, loose views after it', async () => {
    await withFolder();
    expect(row('Work').textContent).toContain('1');
    expect(row('Beta').getAttribute('style')).toContain('--depth: 1');
    expect(row('Alpha').getAttribute('style')).toContain('--depth: 0');
    const order = [...document.querySelectorAll('[aria-label=Views] ul .row')].map((r) => r.textContent?.replace(/\d+\s*$/, '').trim());
    expect(order).toEqual(['Work', 'Beta', 'Alpha', 'Gamma']);
  });

  it('collapses a folder and remembers it on this device', async () => {
    await withFolder();
    row('Work').querySelector<HTMLButtonElement>('button.folder')!.click();
    await settle();
    expect(document.body.textContent).not.toContain('Beta');
    expect(JSON.parse(localStorage.getItem('folders.closed')!)).toEqual({ 1: true });
    unmount(app);
    await open(mock);
    expect(document.body.textContent).not.toContain('Beta');
  });

  it('dragging a view onto a folder moves it in; between rows it reorders and takes the folder of that row', async () => {
    const api = await withFolder();
    dragOnto('Alpha', 'Work');
    await settle();
    expect(await folderOf(api, 'Alpha')).toBe(1);
    expect(row('Work').getAttribute('data-into')).toBeNull(); // the highlight ends with the drag
    dragOnto('Beta', 'Gamma', 0.9);
    await settle();
    expect(await folderOf(api, 'Beta')).toBeNull();
    expect(names('[aria-label=Views]')).toEqual(['Alpha', 'Gamma', 'Beta']);
  });

  it('the insertion line and the target folder show while dragging, and Esc puts everything back', async () => {
    const api = await withFolder();
    const [from, to] = [row('Alpha'), row('Gamma')];
    to.getBoundingClientRect = () => ({ top: 0, height: 20 }) as DOMRect;
    fire(from, 'pointerdown', { clientX: 0, clientY: 0 });
    fire(to, 'pointermove', { clientX: 20, clientY: 18 });
    await settle();
    expect(to.getAttribute('data-line')).toBe('after');
    expect(document.querySelector('.dnd-ghost')!.textContent).toBe('Alpha');
    const f = row('Work');
    f.getBoundingClientRect = () => ({ top: 0, height: 20 }) as DOMRect;
    fire(f, 'pointermove', { clientX: 20, clientY: 10 });
    await settle();
    expect(f.getAttribute('data-into')).toBe('');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    fire(f, 'pointerup', { clientX: 20, clientY: 10 });
    await settle();
    expect(document.querySelector('.dnd-ghost')).toBeNull();
    expect(await folderOf(api, 'Alpha')).toBeNull();
  });

  it('holding a view over a collapsed folder opens it', async () => {
    await withFolder();
    row('Work').querySelector<HTMLButtonElement>('button.folder')!.click();
    await settle();
    vi.useFakeTimers();
    const f = row('Work');
    f.getBoundingClientRect = () => ({ top: 0, height: 20 }) as DOMRect;
    fire(row('Alpha'), 'pointerdown', { clientX: 0, clientY: 0 });
    fire(f, 'pointermove', { clientX: 20, clientY: 10 });
    vi.advanceTimersByTime(700);
    vi.useRealTimers();
    await settle();
    expect(document.body.textContent).toContain('Beta');
    fire(f, 'pointerup', { clientX: 20, clientY: 10 });
    await settle();
  });

  it('a folder goes inside another folder, two levels down, and the third level is refused', async () => {
    const api = createMockApi({ ...seed, viewFolders: [folder(1, 'A', 0), folder(2, 'B', 1), folder(3, 'C', 2), folder(4, 'D', 3)] });
    await open(api);
    dragOnto('B', 'A'); // A > B
    await settle();
    dragOnto('C', 'B'); // A > B > C
    await settle();
    expect((await api.folders.list()).map((f) => [f.name, f.parent_id])).toEqual([['A', null], ['B', 1], ['C', 2], ['D', null]]);
    expect(row('C').getAttribute('style')).toContain('--depth: 2');
    dragOnto('D', 'C'); // would be four deep: no target, nothing happens
    await settle();
    expect((await api.folders.list()).find((f) => f.name === 'D')!.parent_id).toBeNull();
  });

  it('deleting a folder keeps its views at the top level', async () => {
    const api = await withFolder();
    await pick('Work', /^Delete/);
    // The shell owns the confirm dialog; the row only asks for it.
    expect(rowUi.dialog).toMatchObject({ mode: 'delete', target: { kind: 'folder', id: 1 } });
    await remove(rowUi.dialog!.target);
    expect(await api.folders.list()).toEqual([]);
    expect((await api.views.list()).every((v) => v.folder_id === null)).toBe(true);
  });

  it('the Move to folder submenu lists the folders by depth, and moving a project uses it', async () => {
    const api = createMockApi({ ...seed, viewFolders: [folder(1, 'Work', 0, null, 'project'), folder(2, 'Deep', 1, 1, 'project'), folder(3, 'Other', 2, null, 'view')] });
    await open(api);
    const name = fixtures.projects[0].name;
    document.querySelector(`.rowactions[aria-label="${name}"]`)!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    flushSync();
    await vi.waitFor(() => expect([...document.querySelectorAll('[role="menuitem"]')].some((e) => /Move to folder/.test(e.textContent ?? ''))).toBe(true));
    const trigger = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((e) => /Move to folder/.test(e.textContent ?? ''))!;
    trigger.focus();
    trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await vi.waitFor(() => expect([...document.querySelectorAll('[role="menuitem"]')].map((e) => e.textContent?.trim())).toContain('Deep'));
    expect([...document.querySelectorAll('[role="menuitem"]')].map((e) => e.textContent?.trim())).toEqual(expect.arrayContaining(['Top level', 'Work', 'Deep', 'New folder…']));
    expect([...document.querySelectorAll('[role="menuitem"]')].map((e) => e.textContent?.trim())).not.toContain('Other');
    menuItem(/^Deep$/).click();
    await settle();
    expect((await api.folders.items()).find((p) => p.section === 'project' && p.id === fixtures.projects[0].id)!.folder_id).toBe(2);
    expect(row(name).getAttribute('style')).toContain('--depth: 2');
  });

  it('a folder in Pinned and one in Projects work like the one in Views', async () => {
    const api = createMockApi({ ...seed, viewFolders: [folder(1, 'Fav', 0, null, 'pin'), folder(2, 'Work', 1, null, 'project')] });
    await api.pins.pin('View', '1', 'One');
    await open(api);
    dragOnto('One', 'Fav');
    dragOnto(fixtures.projects[0].name, 'Work');
    await settle();
    const at = (await api.folders.items()).filter((p) => p.folder_id !== null).map((p) => [p.section, p.folder_id]);
    expect(at).toEqual(expect.arrayContaining([['pin', 1], ['project', 2]]));
  });

  it('a section menu and right-click on its empty space make a folder in that section', async () => {
    const api = await withFolder();
    const blank = document.querySelector('[aria-label=Projects] .blank')!;
    blank.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    flushSync();
    await vi.waitFor(() => menuItem(/^New folder$/).click());
    await settle();
    expect((await api.folders.list()).map((f) => f.section)).toEqual(['view', 'project']);
    expect(rowUi.dialog).toMatchObject({ mode: 'rename', target: { kind: 'folder' } });
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
  beforeEach(() => setTooltips(true));
  afterEach(() => setTooltips(false));
  const tip = async (el: Element | null) => {
    (el as HTMLElement).focus();
    await settle();
    return document.querySelector('.cx-tip')?.textContent ?? '';
  };

  it('a row shows no action button on hover: it only highlights, the menu is on right-click', async () => {
    await open();
    expect(document.querySelector('[aria-label^="More actions for"]')).toBeNull();
  });

  it('a section link shows the go-to key', async () => {
    await open();
    const t = await tip([...document.querySelectorAll('nav a')].find((a) => a.textContent?.includes('My issues'))!);
    expect(t).toContain('My issues');
    expect(t).toMatch(/G\s*M|GM/);
  });
});

describe('Sidebar project views, pinned folders and the projects switch', () => {
  const pv = { id: 9, name: 'Roadmap', def, position: 3, icon: null, color: null, entity: 'project' as const };
  const withViews = (extra: object = {}) => createMockApi({ ...seed, views: [...seed.views, pv], ...extra });
  afterEach(() => localStorage.clear());

  it('a project view sits among the views, not under Projects', async () => {
    await open(withViews());
    expect(names('[aria-label=Views]')).toEqual(['Alpha', 'Beta', 'Gamma', 'Roadmap']);
    expect(document.querySelector('[aria-label=Projects] [data-project-view]')).toBeNull();
    expect(row('Roadmap').querySelector('[aria-label="Project view"]')).not.toBeNull();
    expect(row('Alpha').querySelector('[aria-label="Project view"]')).toBeNull();
  });

  it('a project view moves into a view folder by drag', async () => {
    const api = withViews({ viewFolders: [folder(1, 'Work')] });
    await open(api);
    dragOnto('Roadmap', 'Work');
    await settle();
    expect((await api.folders.items()).find((p) => p.section === 'view' && p.id === 9)!.folder_id).toBe(1);
    expect(row('Roadmap').getAttribute('style')).toContain('--depth: 1');
  });

  it('projects are hidden by default and a right-click switch shows them, remembered on this device', async () => {
    sidebarPrefs.showProjects = false;
    await open();
    const first = fixtures.projects[0].name;
    expect(names('[aria-label=Projects]')).toEqual(['All projects']);
    document.querySelector('[aria-label=Projects] .head')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    flushSync();
    await vi.waitFor(() => menuItem(/^Show projects in the sidebar$/).click());
    await settle();
    expect(names('[aria-label=Projects]')).toContain(first);
    expect(localStorage.getItem('sidebar.showProjects')).toBe('1');
  });

  it('a pinned folder lists its views live, and a pinned project view shows in Pinned', async () => {
    const api = withViews({ viewFolders: [folder(1, 'Work')], views: [...seed.views, pv].map((v) => ({ ...v, folder_id: v.id === 9 ? 1 : null, template_id: null })) });
    await open(api);
    await pick('Work', /^Pin Work$/);
    await pick('Roadmap', /^Pin Roadmap$/);
    const pinned = () => [...document.querySelectorAll('[aria-label=Pinned] .row')].map((r) => r.textContent?.replace(/\s+/g, ' ').trim());
    expect(pinned()).toEqual(expect.arrayContaining([expect.stringContaining('Work'), expect.stringContaining('Roadmap')]));
    // the folder's contents are in Pinned too, not only the pin of the view
    expect(pinned().filter((t) => t?.includes('Roadmap'))).toHaveLength(2);
    // live: a view added to the folder shows up without pinning again
    await api.folders.placeItem('view', 1, 1, null);
    await refreshAll();
    await settle();
    expect(pinned().filter((t) => t?.includes('Alpha'))).toHaveLength(1);
    await pick('Work', /^Unpin Work$/);
    expect(pinned().filter((t) => t?.includes('Alpha'))).toHaveLength(0);
  });
});
