import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import RowActions from './RowActions.svelte';

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

const row = (label = 'Alpha') => createRawSnippet(() => ({ render: () => `<span>${label}</span>` }));
const actions = () => {
  const run = vi.fn();
  return { run, list: [
    { id: 'rename', label: 'Rename Alpha', icon: 'edit', run },
    { id: 'delete', label: 'Delete Alpha', icon: 'trash', danger: true, run },
  ] };
};
const more = () => document.querySelector('button[aria-label="More actions for Alpha"]') as HTMLButtonElement;
const items = () => [...document.querySelectorAll('[role="menuitem"]')].map((e) => e.textContent?.trim());

describe('RowActions', () => {
  it('row_actions_show_one_more_button', () => {
    const { list } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    expect([...document.querySelectorAll('button')].map((b) => b.getAttribute('aria-label'))).toEqual(['More actions for Alpha']);
  });

  it('more_button_opens_the_actions_menu', async () => {
    const { list, run } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    more().click();
    flushSync();
    await vi.waitFor(() => expect(items()).toEqual(['Rename Alpha', 'Delete Alpha']));
    expect(run).not.toHaveBeenCalled();
  });

  it('choosing_a_menu_item_runs_it', async () => {
    const { list, run } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    more().click();
    flushSync();
    await vi.waitFor(() => expect(items()).toEqual(['Rename Alpha', 'Delete Alpha']));
    (document.querySelectorAll('[role="menuitem"]')[0] as HTMLElement).click();
    expect(run).toHaveBeenCalledOnce();
  });

  it('shift_f10_opens_the_same_actions_as_a_menu', async () => {
    const { list } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    document.querySelector('[role=group]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true, cancelable: true }));
    flushSync();
    await vi.waitFor(() => expect(items()).toEqual(['Rename Alpha', 'Delete Alpha']));
  });
});
