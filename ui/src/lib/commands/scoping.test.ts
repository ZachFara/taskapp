import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { attachKeys, type Ctx, type Layout } from './index';
import { selection } from '../selection.svelte';

// Real key engine, real command defs: which command answers a key depends on the layout and on an open issue.
const press = (key: string, init: KeyboardEventInit = {}) =>
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
const api = { issues: { get: async (id: number) => ({ id, key: `ABC-${id}` }) } } as unknown as Ctx['api'];

let detach: () => void;
let ctx: Ctx;
let events: { name: string; detail: unknown }[];
const seen = ['command:palette.open', 'command:issue.detail.edit', 'command:table.right'];
const listeners = seen.map((name) => {
  const f = (e: Event) => events.push({ name, detail: (e as CustomEvent).detail });
  return [name, f] as const;
});
const view = (layout: Layout | undefined, over: Partial<Ctx> = {}): Ctx => ({
  route: '/view/1', layout, selection: [], focusedIssueId: selection.focused, openIssueId: null, api, navigate: () => {}, ...over,
});

beforeEach(() => {
  events = [];
  selection.clear();
  selection.focus(1);
  selection.setColumns([[1, 2], [3, 4]]);
  listeners.forEach(([n, f]) => addEventListener(n, f));
  detach = attachKeys(() => ctx);
});
afterEach(() => {
  detach();
  listeners.forEach(([n, f]) => removeEventListener(n, f));
  document.body.innerHTML = '';
});
const labelsPage = { name: 'command:palette.open', detail: { page: 'issue.labels' } };

describe('l and shift+l', () => {
  it('on the board h and l move between columns and l does not open labels', () => {
    ctx = view('board');
    press('l');
    expect(selection.focused).toBe(3);
    press('h');
    expect(selection.focused).toBe(1);
    expect(events).toEqual([]);
  });

  it.each<Layout>(['list', 'table'])('in the %s l opens the labels page and focus stays', (layout) => {
    ctx = view(layout);
    press('l');
    expect(events).toEqual([labelsPage]);
    expect(selection.focused).toBe(1);
  });

  it('in an open issue l edits the labels there, not the bulk page', () => {
    ctx = view('list', { route: '/issue/ABC-1', openIssueId: 1 });
    press('l');
    expect(events).toEqual([{ name: 'command:issue.detail.edit', detail: 'labels' }]);
  });

  it.each<[string, Ctx]>([
    ['board', view('board')],
    ['list', view('list')],
    ['table', view('table')],
    ['an open issue', view('list', { route: '/issue/ABC-1', openIssueId: 1 })],
  ])('shift+l always opens the labels page: %s', (_n, c) => {
    ctx = c;
    press('L', { shiftKey: true });
    expect(events).toEqual([labelsPage]);
  });

  it('the table keeps arrowright for its next column', () => {
    ctx = view('table');
    press('ArrowRight');
    expect(events).toEqual([{ name: 'command:table.right', detail: null }]);
  });
});

describe('layout scopes', () => {
  it('t goes to today in the calendar and in the timeline, each in its own layout', () => {
    const today = vi.fn();
    addEventListener('command:calendar.today', today);
    ctx = view('calendar');
    press('t');
    expect(today).toHaveBeenCalledTimes(1);
    ctx = view('list');
    press('t');
    expect(today).toHaveBeenCalledTimes(1);
    removeEventListener('command:calendar.today', today);
  });

  it('an open issue wins [ and ] over the calendar', () => {
    const issue = vi.fn();
    const month = vi.fn();
    addEventListener('command:issue.detail.next', issue);
    addEventListener('command:calendar.next', month);
    ctx = view('calendar', { openIssueId: 1 });
    press(']');
    expect([issue.mock.calls.length, month.mock.calls.length]).toEqual([1, 0]);
    ctx = view('calendar');
    press(']');
    expect([issue.mock.calls.length, month.mock.calls.length]).toEqual([1, 1]);
    removeEventListener('command:issue.detail.next', issue);
    removeEventListener('command:calendar.next', month);
  });

  it('in the table j and k are the table\'s, not a second move through the selection', () => {
    ctx = view('table');
    const next = vi.fn();
    addEventListener('command:table.next', next);
    press('j');
    expect(next).toHaveBeenCalledTimes(1);
    expect(selection.focused).toBe(1);
    removeEventListener('command:table.next', next);
  });
});
