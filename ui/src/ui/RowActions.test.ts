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

describe('RowActions', () => {
  it('row_actions_are_buttons_labelled_by_their_action', () => {
    const { list } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    expect(document.querySelector('[role=group][aria-label=Alpha]')).not.toBeNull();
    expect([...document.querySelectorAll('button')].map((b) => b.getAttribute('aria-label'))).toEqual(['Rename Alpha', 'Delete Alpha']);
  });

  it('clicking_an_action_runs_it', () => {
    const { list, run } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    (document.querySelector('button[aria-label="Rename Alpha"]') as HTMLButtonElement).click();
    expect(run).toHaveBeenCalledOnce();
  });

  it('shift_f10_opens_the_same_actions_as_a_menu', async () => {
    const { list } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, children: row() } });
    flushSync();
    document.querySelector('[role=group]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true, cancelable: true }));
    flushSync();
    await vi.waitFor(() => expect([...document.querySelectorAll('[role="menuitem"]')].map((e) => e.textContent?.trim())).toEqual(['Rename Alpha', 'Delete Alpha']));
  });

  it('menu_false_has_no_context_menu', () => {
    const { list } = actions();
    app = mount(RowActions, { target: document.body, props: { label: 'Alpha', actions: list, menu: false, children: row() } });
    flushSync();
    document.querySelector('[role=group]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true }));
    flushSync();
    expect(document.querySelector('[role="menu"]')).toBeNull();
  });
});
