import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createKeys, type Binding } from './keys';

const press = (target: Element, key: string, init: KeyboardEventInit = {}) => {
  const e = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(e);
  return e;
};
let detach: () => void;
const setup = (list: Binding[]) => (detach = createKeys(() => list));
const bind = (seq: string, extra: Partial<Binding> = {}) => {
  const run = vi.fn();
  return { b: { seq, run, ...extra } as Binding, run };
};
beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  detach();
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('keys', () => {
  it('fires single keys, modifiers and shift variants', () => {
    const c = bind('c'), k = bind('mod+k'), j = bind('shift+j'), G = bind('G');
    setup([c.b, k.b, j.b, G.b]);
    press(document.body, 'c');
    press(document.body, 'k', { metaKey: true });
    press(document.body, 'J', { shiftKey: true });
    press(document.body, 'G', { shiftKey: true });
    for (const x of [c, k, j, G]) expect(x.run).toHaveBeenCalledTimes(1);
  });

  it('sequences fire at the end only and time out after 1s', () => {
    const gi = bind('g i'), gg = bind('g g');
    setup([gi.b, gg.b]);
    press(document.body, 'i');
    press(document.body, 'g');
    press(document.body, 'i');
    expect(gi.run).toHaveBeenCalledTimes(1);
    press(document.body, 'g');
    vi.advanceTimersByTime(1001);
    press(document.body, 'g');
    expect(gg.run).not.toHaveBeenCalled();
    press(document.body, 'g');
    expect(gg.run).toHaveBeenCalledTimes(1);
  });

  it('Escape cancels a pending sequence', () => {
    const gi = bind('g i');
    setup([gi.b]);
    press(document.body, 'g');
    press(document.body, 'Escape');
    press(document.body, 'i');
    expect(gi.run).not.toHaveBeenCalled();
  });

  it('a count before the key is passed to the handler', () => {
    const j = bind('j');
    setup([j.b]);
    press(document.body, '5');
    press(document.body, 'j');
    press(document.body, 'j');
    expect(j.run.mock.calls).toEqual([[5], [1]]);
  });

  it.each([
    ['input', '<input>'],
    ['textarea', '<textarea></textarea>'],
    ['select', '<select></select>'],
    ['contenteditable', '<div contenteditable="true"></div>'],
  ])('ignores plain keys in a %s but allows Escape and global mod combos', (_, html) => {
    const c = bind('c'), esc = bind('escape'), k = bind('mod+k', { global: true }), p = bind('mod+p');
    setup([c.b, esc.b, k.b, p.b]);
    document.body.innerHTML = html;
    const el = document.body.firstElementChild!;
    const e = press(el, 'c');
    press(el, 'Escape');
    press(el, 'k', { metaKey: true });
    press(el, 'p', { metaKey: true });
    expect(c.run).not.toHaveBeenCalled();
    expect(e.defaultPrevented).toBe(false);
    expect(esc.run).toHaveBeenCalledTimes(1);
    expect(k.run).toHaveBeenCalledTimes(1);
    expect(p.run).not.toHaveBeenCalled();
  });

  it('the key that fires is not typed into the input it opens', () => {
    const c = bind('c');
    setup([c.b]);
    expect(press(document.body, 'c').defaultPrevented).toBe(true);
  });

  it('ignores IME composition', () => {
    const c = bind('c');
    setup([c.b]);
    press(document.body, 'c', { isComposing: true });
    expect(c.run).not.toHaveBeenCalled();
  });

  it('Enter is left to a focused button', () => {
    const en = bind('enter');
    setup([en.b]);
    document.body.innerHTML = '<button></button>';
    press(document.body.firstElementChild!, 'Enter');
    expect(en.run).not.toHaveBeenCalled();
  });
});
