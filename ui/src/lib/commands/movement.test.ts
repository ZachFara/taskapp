import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { attachKeys, type Ctx } from './index';
import { selection } from '../selection.svelte';
import { selectedIssueId } from '../stores.svelte';

const press = (target: Element, key: string, init: KeyboardEventInit = {}) =>
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));

let detach: () => void;
let navigate: ReturnType<typeof vi.fn>;
let api: { issues: { get: ReturnType<typeof vi.fn> } };
const ctx = (over: Partial<Ctx> = {}): Ctx => ({
  route: '/view/1',
  selection: selection.selected,
  focusedIssueId: selection.focused,
  openIssueId: null,
  api: api as unknown as Ctx['api'],
  navigate,
  ...over,
});

beforeEach(() => {
  navigate = vi.fn();
  api = { issues: { get: vi.fn(async (id: number) => ({ id, key: `ABC-${id}` })) } };
  selection.focus(null);
  selection.clear();
  selection.setColumns([[1, 2, 3, 4]]);
  detach = attachKeys(ctx);
});
afterEach(() => {
  detach();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('movement keys', () => {
  it('j and k move focus down and up, clamped at the ends', () => {
    press(document.body, 'j');
    expect(selection.focused).toBe(1);
    press(document.body, 'j');
    expect(selection.focused).toBe(2);
    press(document.body, 'k');
    expect(selection.focused).toBe(1);
    press(document.body, 'k');
    expect(selection.focused).toBe(1);
  });

  it('g g goes to the top and G to the bottom', () => {
    selection.focus(3);
    press(document.body, 'G', { shiftKey: true });
    expect(selection.focused).toBe(4);
    press(document.body, 'g');
    press(document.body, 'g');
    expect(selection.focused).toBe(1);
  });

  it('shift+j and shift+k extend the selection from the anchor', () => {
    selection.focus(1);
    press(document.body, 'J', { shiftKey: true });
    press(document.body, 'J', { shiftKey: true });
    expect(selection.selected).toEqual([1, 2, 3]);
    press(document.body, 'K', { shiftKey: true });
    expect(selection.selected).toEqual([1, 2]);
  });

  it('x toggles the focused row and mod+a selects all in view', () => {
    selection.focus(2);
    press(document.body, 'x');
    expect(selection.selected).toEqual([2]);
    press(document.body, 'x');
    expect(selection.selected).toEqual([]);
    press(document.body, 'a', { metaKey: true });
    expect(selection.selected).toEqual([1, 2, 3, 4]);
  });

  it('h and l move focus between board columns, keeping the row', () => {
    selection.setColumns([[1, 2], [3, 4]]);
    selection.focus(2);
    press(document.body, 'l');
    expect(selection.focused).toBe(4);
    press(document.body, 'h');
    expect(selection.focused).toBe(2);
  });

  it('space toggles the peek on the focused row', () => {
    selection.focus(3);
    press(document.body, ' ');
    expect(selection.peeked).toBe(3);
    press(document.body, ' ');
    expect(selection.peeked).toBe(null);
  });

  it('enter opens the focused issue in the side panel, not the full page', () => {
    selection.focus(2);
    press(document.body, 'Enter');
    expect(selectedIssueId.value).toBe(2);
    expect(navigate).not.toHaveBeenCalled();
    selectedIssueId.value = null;
  });

  it('mod+enter opens the full page', async () => {
    selection.focus(2);
    press(document.body, 'Enter', { metaKey: true });
    await vi.waitFor(() => expect(navigate).toHaveBeenCalledWith('/issue/ABC-2'));
  });

  it('a count moves that many rows', () => {
    press(document.body, '3');
    press(document.body, 'j');
    expect(selection.focused).toBe(3);
  });

  it('escape clears the selection; it never steps back out of the app; an open issue is left to issue.detail.close', () => {
    const back = vi.spyOn(history, 'back').mockImplementation(() => {});
    selection.selectAll([1, 2]);
    press(document.body, 'Escape');
    expect(back).not.toHaveBeenCalled();
    expect(selection.selected).toEqual([]);
    detach();
    detach = attachKeys(() => ctx({ openIssueId: 2 }));
    press(document.body, 'Escape');
    expect(back).not.toHaveBeenCalled();
    detach();
    detach = attachKeys(() => ctx({ route: '/projects' }));
    press(document.body, 'Escape');
    expect(back).not.toHaveBeenCalled();
  });

  it('keys do not fire while an input has focus', () => {
    const input = document.createElement('input');
    document.body.append(input);
    input.focus();
    press(input, 'j');
    expect(selection.focused).toBe(null);
  });
});
