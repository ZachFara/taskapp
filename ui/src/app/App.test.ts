import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { selectedIssueId } from '../lib/stores.svelte';
import App from './App.svelte';
import { navigate } from './routes';

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const q = <T extends Element = HTMLElement>(s: string) => document.querySelector<T>(s);
const panel = () => q('aside.panel');
const esc = () => document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

async function open(path: string) {
  history.replaceState(null, '', path);
  app = mount(App, { target: document.body });
  await settle();
}
async function pickRow(key = 'ZF-1') {
  q(`[data-key=${key}]`)!.click();
  await settle();
}

beforeEach(() => {
  setApi(createMockApi());
  selectedIssueId.value = null;
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('App shell', () => {
  it('root_path_opens_a_page_not_not_found', async () => {
    await open('/');
    expect(document.body.textContent).not.toContain('Not found');
  });

  it('renders_sidebar_and_page_inside_shell', async () => {
    await open('/view/1');
    expect(q('aside.sidebar')!.textContent).toContain('Active');
    expect(q('main [role=listbox]')).not.toBeNull();
  });

  it('click_row_opens_detail_in_panel', async () => {
    await open('/view/1');
    await pickRow();
    expect(panel()!.querySelector<HTMLInputElement>('[aria-label=Title]')!.value).toBe('Fixture task 1');
  });

  it('esc_closes_panel_not_page', async () => {
    await open('/view/1');
    await pickRow();
    esc();
    await settle();
    expect(panel()!.textContent).toBe('');
    expect(location.pathname).toBe('/view/1');
  });

  it('esc_with_palette_open_keeps_panel', async () => {
    await open('/view/1');
    await pickRow();
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }));
    await settle();
    expect(q('[role=dialog]')).not.toBeNull();
    q<HTMLInputElement>('[role=dialog] input')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    expect(q('[role=dialog]')).toBeNull();
    expect(panel()!.querySelector('[aria-label=Title]')).not.toBeNull();
  });

  it('navigating_to_another_view_keeps_panel_closed', async () => {
    await open('/view/1');
    await pickRow();
    navigate('/view/2');
    await settle();
    expect(panel()!.textContent).toBe('');
  });

  it('vanished_issue_shows_not_found_in_panel', async () => {
    await open('/view/1');
    selectedIssueId.value = 99999;
    await settle();
    expect(panel()!.querySelector('[role=alert]')!.textContent).toContain('issue #99999');
  });

  it('rename_view_saves_and_refreshes_the_sidebar', async () => {
    await open('/view/1');
    q<HTMLButtonElement>('button[aria-label="Rename view"]')!.click();
    await settle();
    const name = q<HTMLInputElement>('[aria-label="View name"]')!;
    name.value = 'Renamed';
    name.dispatchEvent(new Event('input', { bubbles: true }));
    name.form!.requestSubmit();
    await settle();
    expect(q('aside.sidebar')!.textContent).toContain('Renamed');
  });

  it('palette_opens_from_any_page', async () => {
    await open('/settings/general');
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }));
    await settle();
    expect(q('[role=dialog][aria-label="Command palette"]')).not.toBeNull();
  });
});
