import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineCommand } from '../lib/commands/registry';
import CheatSheet from './CheatSheet.svelte';

defineCommand({
  id: 'test.cheatsheet.entry',
  label: 'Test cheat sheet entry',
  icon: 'plus',
  group: 'Testing',
  shortcut: 'g x',
  run: () => {},
});

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const open = async () => {
  app = mount(CheatSheet, { target: document.body, props: { open: true } });
  await settle();
};
const search = () => document.querySelector<HTMLInputElement>('input[type=search], input')!;
const rowFor = (label: string) => [...document.querySelectorAll('li')].find((li) => li.textContent?.includes(label));

afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('CheatSheet', () => {
  it('lists_a_newly_registered_command_with_its_keys_as_separate_keys', async () => {
    await open();
    const row = rowFor('Test cheat sheet entry');
    expect(row).toBeTruthy();
    const keys = [...row!.querySelectorAll('kbd')].map((k) => k.textContent);
    expect(keys).toEqual(['g', 'x']);
  });

  it('lists_commands_from_the_registry_not_a_hand_written_list', async () => {
    await open();
    expect(rowFor('Create issue')).toBeTruthy();
    expect(rowFor('Keyboard shortcuts')).toBeTruthy();
  });

  it('filter_narrows_the_list', async () => {
    await open();
    search().value = 'cheat';
    search().dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(rowFor('Test cheat sheet entry')).toBeTruthy();
    expect(rowFor('Create issue')).toBeFalsy();
  });

  it('filter_with_no_match_shows_an_empty_state', async () => {
    await open();
    search().value = 'zzzz-no-such-command';
    search().dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    expect(document.querySelectorAll('li[data-row]').length).toBe(0);
    expect(document.body.textContent).toContain('No shortcuts match');
  });

  it('escape_closes_the_sheet', async () => {
    const onopenchange = vi.fn();
    app = mount(CheatSheet, { target: document.body, props: { open: true, onopenchange } });
    await settle();
    search().dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    expect(document.querySelector('[role=dialog]')).toBeNull();
  });

  it('opens_when_the_help_shortcuts_command_fires', async () => {
    app = mount(CheatSheet, { target: document.body, props: { open: false } });
    await settle();
    expect(document.querySelector('[role=dialog]')).toBeNull();
    window.dispatchEvent(new CustomEvent('command:help.shortcuts'));
    await settle();
    expect(document.querySelector('[role=dialog]')).not.toBeNull();
  });
});
