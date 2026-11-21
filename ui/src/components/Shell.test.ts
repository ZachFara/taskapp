import { createRawSnippet, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import Shell from './Shell.svelte';

let app: ReturnType<typeof mount> | null = null;
const body = createRawSnippet(() => ({ render: () => '<p>main view</p>' }));
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const press = (target: EventTarget, key: string, init: KeyboardEventInit = {}) =>
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...init }));

beforeEach(() => setApi(createMockApi()));
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

async function open() {
  app = mount(Shell, { target: document.body, props: { children: body } });
  await settle();
}

describe('Shell', () => {
  it('cmd_k_opens_and_esc_closes', async () => {
    await open();
    expect(document.querySelector('[role=dialog]')).toBeNull();
    press(window, 'k', { metaKey: true });
    await settle();
    const input = document.querySelector<HTMLInputElement>('[role=dialog] input')!;
    expect(input).not.toBeNull();
    press(input, 'Escape');
    await settle();
    expect(document.querySelector('[role=dialog]')).toBeNull();
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

  it('slash_opens_palette_and_g_i_g_v_navigate', async () => {
    history.replaceState(null, '', '/');
    await open();
    press(window, '/');
    await settle();
    expect(document.querySelector('[role=dialog] input')).not.toBeNull();
    press(document.querySelector('[role=dialog] input')!, 'Escape');
    await settle();
    press(window, 'g');
    press(window, 'i');
    expect(location.pathname).toBe('/view/1');
    history.replaceState(null, '', '/');
    press(window, 'g');
    press(window, 'v');
    await settle();
    expect(document.querySelector('[role=dialog]')).not.toBeNull();
  });
});
