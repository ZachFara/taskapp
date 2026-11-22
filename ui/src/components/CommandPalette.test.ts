import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { defineCommand } from '../lib/commands/registry';
import { selection } from '../lib/selection.svelte';
import { refreshAll } from '../lib/stores.svelte';
import CommandPalette from './CommandPalette.svelte';

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const press = (key: string, opts: KeyboardEventInit = {}) =>
  input().dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...opts }));
const input = () => document.querySelector<HTMLInputElement>('input')!;
async function type(text: string) {
  input().value = text;
  input().dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
const rows = () => [...document.querySelectorAll('[role=option]')].map((x) => x.textContent ?? '');
const active = () => document.querySelector('[role=option][aria-selected=true]')?.textContent ?? '';

const onclose = vi.fn();
const oncreate = vi.fn();
const ran = vi.fn();
const picked = vi.fn();
const checked = new Set<string>();

// Commands owned by this test so nothing depends on what other screens register.
defineCommand({ id: 'test.plain', label: 'Zebra plain', icon: 'star', group: 'Test', shortcut: 'alt+z', run: () => ran('plain') });
defineCommand({
  id: 'test.ctx', label: 'Zebra on selection', icon: 'flag', group: 'Issue', noShortcutReason: 'test',
  when: (c) => c.selection.length > 0, run: (c) => ran('ctx', c.selection),
});
defineCommand({
  id: 'test.page', label: 'Zebra set status', icon: 'circle', group: 'Test', noShortcutReason: 'test',
  page: () => ({ title: 'Set status', items: [
    { id: 'a', label: 'Alpha', run: () => picked('Alpha') },
    { id: 'b', label: 'Beta', run: () => picked('Beta') },
  ] }),
});
defineCommand({
  id: 'test.multi', label: 'Zebra toggle labels', icon: 'tag', group: 'Test', noShortcutReason: 'test',
  page: () => ({ title: 'Labels', multi: true, items: ['x', 'y'].map((id) => ({
    id, label: `Label ${id}`, checked: () => checked.has(id),
    run: () => void (checked.has(id) ? checked.delete(id) : checked.add(id)),
  })) }),
} as never);
defineCommand({
  id: 'test.free', label: 'Zebra set due', icon: 'calendar', group: 'Test', noShortcutReason: 'test',
  page: () => ({ title: 'Due date', items: [], parse: (t: string) => (t === 'fri' ? { label: 'Due Friday', run: () => picked('fri') } : null) }),
} as never);
let release: (() => void) | undefined;
defineCommand({
  id: 'test.slow', label: 'Zebra slow page', icon: 'tag', group: 'Test', noShortcutReason: 'test',
  page: () => new Promise((done) => { release = () => done({ title: 'Slow', items: [{ id: 'k', label: 'Kappa', checked: true, run: () => {} }] }); }),
} as never);

beforeEach(async () => {
  setApi(createMockApi());
  await refreshAll();
  onclose.mockClear(); oncreate.mockClear(); ran.mockClear(); picked.mockClear();
  checked.clear();
  selection.clear();
  app = mount(CommandPalette, { target: document.body, props: { onclose, oncreate } });
  await settle();
});
afterEach(() => {
  unmount(app!);
  document.body.innerHTML = '';
  selection.clear();
  history.pushState(null, '', '/');
});

describe('CommandPalette', () => {
  it('lists_commands_in_a_listbox_with_the_registry_shortcut', async () => {
    await type('zebra plain');
    expect(document.querySelector('[role=listbox]')).not.toBeNull();
    expect(rows()[0]).toContain('Zebra plain');
    expect(document.querySelector('[role=option] kbd')?.textContent).toBe('alt+z');
  });

  it('issue_actions_show_under_a_selection_header', async () => {
    const a = await api.issues.create({ title: 'P1' });
    const b = await api.issues.create({ title: 'P2' });
    selection.selectAll([a.id, b.id]);
    unmount(app!);
    app = mount(CommandPalette, { target: document.body, props: { onclose, oncreate } });
    await settle();
    expect(document.body.textContent).toContain('2 issues selected');
    await type('zebra on selection');
    expect(rows()[0]).toContain('Zebra on selection');
    press('Enter');
    await settle();
    expect(ran).toHaveBeenCalledWith('ctx', [a.id, b.id]);
    expect(onclose).toHaveBeenCalled();
  });

  it('a start page opens the menu straight at that page with an empty query', async () => {
    unmount(app!);
    app = mount(CommandPalette, { target: document.body, props: { onclose, oncreate, page: 'test.page' } });
    await settle();
    expect(document.querySelector('.crumb')?.textContent).toContain('Set status');
    expect(input().value).toBe('');
    expect(rows()).toEqual([expect.stringContaining('Alpha'), expect.stringContaining('Beta')]);
  });

  it('nested_page_opens_and_backspace_on_empty_goes_back', async () => {
    await type('zebra set status');
    press('Enter');
    await settle();
    expect(document.querySelector('.crumb')?.textContent).toContain('Set status');
    expect(rows()).toEqual([expect.stringContaining('Alpha'), expect.stringContaining('Beta')]);
    press('Backspace');
    await settle();
    expect(document.querySelector('.crumb')).toBeNull();
    expect(onclose).not.toHaveBeenCalled();
  });

  it('page_item_runs_and_closes', async () => {
    await type('zebra set status');
    press('Enter');
    await settle();
    press('ArrowDown');
    await settle();
    press('Enter');
    await settle();
    expect(picked).toHaveBeenCalledWith('Beta');
    expect(onclose).toHaveBeenCalled();
  });

  it('escape_closes_one_level_then_the_menu', async () => {
    await type('zebra set status');
    press('Enter');
    await settle();
    press('Escape');
    await settle();
    expect(document.querySelector('.crumb')).toBeNull();
    expect(onclose).not.toHaveBeenCalled();
    press('Escape');
    await settle();
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('issue_search_result_navigates', async () => {
    const i = await api.issues.create({ title: 'Quokka migration' });
    await type('quokka');
    expect(rows().some((r) => r.includes(i.key) && r.includes('Quokka migration'))).toBe(true);
    press('Enter');
    await settle();
    expect(location.pathname).toBe(`/issue/${i.key}`);
  });

  it('multi_pick_page_stays_open_and_shows_checkmarks', async () => {
    await type('zebra toggle');
    press('Enter');
    await settle();
    press('Enter');
    await settle();
    expect(onclose).not.toHaveBeenCalled();
    expect(document.querySelector('[role=option][aria-checked=true]')?.textContent).toContain('Label x');
    press('ArrowDown');
    press('Enter');
    await settle();
    expect(document.querySelectorAll('[role=option][aria-checked=true]').length).toBe(2);
  });

  it('async_page_shows_a_skeleton_then_its_items_with_a_boolean_check', async () => {
    await type('zebra slow');
    press('Enter');
    await settle();
    expect(document.querySelector('.skel [aria-busy], .skel')).not.toBeNull();
    release!();
    await settle();
    expect(document.querySelector('.skel')).toBeNull();
    expect(document.querySelector('[role=option][aria-checked=true]')?.textContent).toContain('Kappa');
  });

  it('free_input_page_previews_the_parsed_value', async () => {
    await type('zebra set due');
    press('Enter');
    await settle();
    await type('fri');
    expect(rows()[0]).toContain('Due Friday');
    press('Enter');
    await settle();
    expect(picked).toHaveBeenCalledWith('fri');
  });

  it('ctrl_n_and_ctrl_p_move_and_tab_completes', async () => {
    await type('zebra');
    const first = active();
    press('n', { ctrlKey: true });
    await settle();
    expect(active()).not.toBe(first);
    press('p', { ctrlKey: true });
    await settle();
    expect(active()).toBe(first);
    press('Tab');
    await settle();
    expect(input().value.toLowerCase()).toContain('zebra');
  });

  it('empty_state_and_enter_does_nothing', async () => {
    await type('qqqzzz');
    expect(document.body.textContent).toContain('No matches');
    press('Enter');
    expect(onclose).not.toHaveBeenCalled();
  });

  it('input_keeps_focus', async () => {
    await type('zebra');
    expect(document.activeElement).toBe(input());
  });
});
