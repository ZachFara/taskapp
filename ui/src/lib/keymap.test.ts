import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerShortcut } from './keymap';

const press = (target: Element, key: string, init: KeyboardEventInit = {}) =>
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...init }));

let off: (() => void)[] = [];
afterEach(() => {
  off.forEach((f) => f());
  off = [];
  document.body.innerHTML = '';
});
const register = (combo: string) => {
  const handler = vi.fn();
  off.push(registerShortcut(combo, combo, handler));
  return handler;
};

describe('keymap', () => {
  it('sequence g then i fires once', () => {
    const handler = register('g i');
    press(document.body, 'g');
    press(document.body, 'i');
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('a lone key of a sequence does not fire it', () => {
    const handler = register('g i');
    press(document.body, 'i');
    expect(handler).not.toHaveBeenCalled();
  });

  it.each([
    ['input', '<input>'],
    ['textarea', '<textarea></textarea>'],
    ['contenteditable', '<div contenteditable="true"></div>'],
  ])('shortcuts ignored while typing in a %s', (_, html) => {
    const handler = register('c');
    document.body.innerHTML = html;
    press(document.body.firstElementChild!, 'c');
    expect(handler).not.toHaveBeenCalled();
  });

  it('cmd+k still fires from a text field', () => {
    const handler = register('cmd+k');
    document.body.innerHTML = '<input>';
    press(document.body.firstElementChild!, 'k', { metaKey: true });
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('unregister stops the handler', () => {
    const handler = register('c');
    off.pop()!();
    press(document.body, 'c');
    expect(handler).not.toHaveBeenCalled();
  });
});
