import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { defineCommand } from '../lib/commands/registry';
import CommandButton from './CommandButton.svelte';

const ran: string[] = [];
defineCommand({
  id: 'test.button.ping',
  label: 'Ping test',
  icon: 'plus',
  group: 'Testing',
  shortcut: 'alt+p',
  run: () => void ran.push('ping'),
});
defineCommand({
  id: 'test.button.hidden',
  label: 'Hidden test',
  icon: 'plus',
  group: 'Testing',
  shortcut: 'alt+h',
  when: () => false,
  run: () => void ran.push('hidden'),
});

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const button = (label: string) => [...document.querySelectorAll('button')].find((b) => b.textContent?.includes(label) || b.getAttribute('aria-label') === label);

beforeEach(() => void ran.splice(0));
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('CommandButton', () => {
  it('shows_the_command_label_and_its_tooltip_with_the_shortcut_on_focus', async () => {
    app = mount(CommandButton, { target: document.body, props: { command: 'test.button.ping' } });
    await settle();
    button('Ping test')!.focus();
    await settle();
    const tip = document.querySelector('.cx-tip');
    expect(tip?.textContent).toContain('Ping test');
    expect([...tip!.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['alt+p']);
  });

  it('click_runs_the_command', async () => {
    app = mount(CommandButton, { target: document.body, props: { command: 'test.button.ping' } });
    await settle();
    button('Ping test')!.click();
    expect(ran).toEqual(['ping']);
  });

  it('icon_only_keeps_the_label_as_its_accessible_name', async () => {
    app = mount(CommandButton, { target: document.body, props: { command: 'test.button.ping', iconOnly: true } });
    await settle();
    expect(document.querySelector('button[aria-label="Ping test"]')).not.toBeNull();
  });

  it('is_hidden_when_its_when_is_false', async () => {
    app = mount(CommandButton, { target: document.body, props: { command: 'test.button.hidden' } });
    await settle();
    expect(document.querySelector('button')).toBeNull();
  });
});
