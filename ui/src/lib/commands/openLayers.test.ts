import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { attachKeys, listCommands, type Ctx, type Layout } from './index';
import { selection } from '../selection.svelte';

const press = (key: string, init: KeyboardEventInit = {}) =>
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
const api = { issues: { get: async (id: number) => ({ id, key: `ABC-${id}` }) } } as unknown as Ctx['api'];
let detach: () => void;
let ctx: Ctx;
const view = (layout: Layout, over: Partial<Ctx> = {}): Ctx => ({
  route: '/view/1', layout, selection: selection.selected, focusedIssueId: selection.focused, openIssueId: null, api, navigate: () => {}, ...over,
});
beforeEach(() => {
  selection.clear();
  selection.focus(null);
  selection.setColumns([[1, 2, 3]]);
  detach = attachKeys(() => ctx);
});
afterEach(() => {
  detach();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('keys while a menu or popover is open', () => {
  it.each(['<div role="menu"></div>', '<div class="cx-surface cx-listbox"></div>', '<div class="cx-surface cx-popover"></div>'])(
    'j does not move rows behind %s',
    (html) => {
      ctx = view('list');
      document.body.insertAdjacentHTML('beforeend', html);
      press('j');
      expect(selection.focused).toBeNull();
    },
  );

  it('j moves rows again once the menu is gone', () => {
    ctx = view('list');
    press('j');
    expect(selection.focused).toBe(1);
  });
});

describe('the open-issue guard in move.escape', () => {
  it('Esc with an issue open leaves the selection to the detail close handler', () => {
    selection.toggle(1);
    ctx = view('list', { openIssueId: 1 });
    press('Escape');
    expect(selection.selected).toEqual([1]);
  });

  // Through the key engine the detail handler outranks this one, so only a direct run shows the guard.
  it('move.escape itself does nothing while an issue is open', () => {
    selection.toggle(1);
    listCommands(view('list'), { all: true }).find((c) => c.id === 'move.escape')!.run!(view('list', { openIssueId: 1 }));
    expect(selection.selected).toEqual([1]);
  });

  it('Esc on a list with nothing open clears the selection', () => {
    selection.toggle(1);
    ctx = view('list');
    press('Escape');
    expect(selection.selected).toEqual([]);
  });
});

describe('chart arrows', () => {
  it('arrow keys on a focused chart move the chart day, not the table cell', () => {
    const seen: string[] = [];
    const on = (n: string) => addEventListener(n, () => seen.push(n));
    on('command:table.right');
    on('command:chart.day');
    document.body.innerHTML = '<div data-chart tabindex="0"></div>';
    document.querySelector<HTMLElement>('[data-chart]')!.focus();
    ctx = view('table');
    press('ArrowRight');
    expect(seen).toEqual(['command:chart.day']);
  });
});
