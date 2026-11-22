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
    const btn = () => document.querySelector<HTMLButtonElement>('[aria-label$="Alpha"]')!;
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
    await api.pins.pin('Project', 'home', 'Home');
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
    await api.pins.pin('Project', 'home', 'Home');
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
    const api = createMockApi(seed);
    await api.pins.pin('Url', 'https://example.com/x', 'Site');
    await api.pins.pin('Url', 'javascript:alert(1)', 'Bad');
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
