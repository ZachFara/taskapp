import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineCommand } from '../lib/commands/registry';
import CheatSheet from './CheatSheet.svelte';

// Own commands only: the registry holds every app command, so tests never assert on other entries.
defineCommand({ id: 'test.cheatsheet.alpha', label: 'Cheat probe alpha', icon: 'plus', group: 'Probe', shortcut: 'g x', run: () => {} });
defineCommand({ id: 'test.cheatsheet.beta', label: 'Cheat probe beta', icon: 'plus', group: 'Probe', shortcut: 'g g', run: () => {} });

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const open = async (props: Record<string, unknown> = {}) => {
  app = mount(CheatSheet, { target: document.body, props: { open: true, ...props } });
  await settle();
};
// The dialog portals to document.body, so every query starts from the dialog itself.
const dialog = () => document.querySelector<HTMLElement>('[role=dialog]');
const search = () => dialog()!.querySelector<HTMLInputElement>('input')!;
const type = async (text: string) => {
  search().value = text;
  search().dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
};
const rowFor = (label: string) => [...(dialog()?.querySelectorAll('li') ?? [])].find((li) => li.textContent?.includes(label));

afterEach(async () => {
  // bits-ui drops its escape layer in an effect teardown, so let that flush before the next test.
  if (app) unmount(app);
  app = null;
  await settle();
  document.body.innerHTML = '';
});

describe('CheatSheet', () => {
  it('lists_a_registered_command_with_its_keys_as_separate_keys', async () => {
    await open();
    const row = rowFor('Cheat probe alpha');
    expect(row).toBeTruthy();
    expect([...row!.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['G', 'X']);
  });

  it('renders_a_repeated_key_in_a_sequence_without_breaking_the_dialog', async () => {
    await open();
    const row = rowFor('Cheat probe beta');
    expect([...row!.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['G', 'G']);
  });

  it('lists_every_registered_command_with_a_shortcut', async () => {
    await open();
    expect(rowFor('Cheat probe alpha')).toBeTruthy();
    expect(rowFor('Cheat probe beta')).toBeTruthy();
  });

  it('filter_narrows_the_list', async () => {
    await open();
    await type('probe alpha');
    expect(rowFor('Cheat probe alpha')).toBeTruthy();
    expect(rowFor('Cheat probe beta')).toBeFalsy();
  });

  it('filter_with_no_match_shows_an_empty_state', async () => {
    await open();
    await type('zzzz-no-such-command');
    expect(dialog()!.querySelectorAll('li[data-row]').length).toBe(0);
    expect(dialog()!.textContent).toContain('No shortcuts match');
  });

  it('escape_closes_the_sheet', async () => {
    const onopenchange = vi.fn();
    await open({ onopenchange });
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    // The node can stay until its exit animation ends, which jsdom never fires; the state is what closed.
    expect(dialog() === null || dialog()!.getAttribute("data-state") === "closed").toBe(true);
  });

  it('opens_when_the_help_shortcuts_command_fires', async () => {
    await open({ open: false });
    expect(dialog()).toBeNull();
    window.dispatchEvent(new CustomEvent('command:help.shortcuts'));
    await settle();
    expect(dialog()).not.toBeNull();
  });
});
