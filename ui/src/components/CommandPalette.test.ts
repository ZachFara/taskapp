import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { refreshAll } from '../lib/stores.svelte';
import CommandPalette from './CommandPalette.svelte';

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const press = (el: Element, key: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
const input = () => document.querySelector<HTMLInputElement>('input')!;
async function type(text: string) {
  input().value = text;
  input().dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
const rows = () => [...document.querySelectorAll('[role=option]')].map((x) => x.textContent);

const onclose = vi.fn();
const oncreate = vi.fn();
beforeEach(async () => {
  setApi(createMockApi());
  await refreshAll();
  onclose.mockClear();
  oncreate.mockClear();
  app = mount(CommandPalette, { target: document.body, props: { onclose, oncreate } });
  await settle();
});
afterEach(() => {
  unmount(app!);
  document.body.innerHTML = '';
  history.pushState(null, '', '/');
});

describe('CommandPalette', () => {
  it('palette_runs_create_command', async () => {
    await type('create');
    press(input(), 'Enter');
    expect(oncreate).toHaveBeenCalledTimes(1);
    expect(onclose).toHaveBeenCalled();
  });

  it('palette_finds_issue_by_key', async () => {
    await type('zf-7');
    expect(rows().some((r) => r?.includes('ZF-7 Fixture task 7'))).toBe(true);
    press(input(), 'Enter');
    expect(location.pathname).toBe('/issue/ZF-7');
  });

  it('arrow_keys_move_the_selection', async () => {
    await type('go to');
    press(input(), 'ArrowDown');
    await settle();
    expect(document.querySelectorAll('[role=option]')[1].getAttribute('aria-selected')).toBe('true');
  });

  it('zero_matches_shows_empty_state_and_enter_does_nothing', async () => {
    await type('qqqzzz');
    expect(document.body.textContent).toContain('No matches');
    press(input(), 'Enter');
    expect(onclose).not.toHaveBeenCalled();
    expect(oncreate).not.toHaveBeenCalled();
    expect(location.pathname).toBe('/');
  });
});
