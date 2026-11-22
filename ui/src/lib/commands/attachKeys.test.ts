import { afterEach, describe, expect, it, vi } from 'vitest';
import { attachKeys } from './index';
import { defineCommand, type Ctx } from './registry';

const ctx = {} as Ctx;
const press = (key: string, init: KeyboardEventInit = {}) =>
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
let detach: () => void;
afterEach(() => {
  detach?.();
  document.body.innerHTML = '';
});

const count = vi.fn();
const plain = vi.fn();
defineCommand({ id: 'test.count', label: 'Count', icon: 'plus', group: 'Test', shortcut: 'Q', run: (_c, n) => count(n) });
defineCommand({ id: 'test.plain', label: 'Plain', icon: 'plus', group: 'Test', shortcut: 'W', run: plain });

describe('attachKeys', () => {
  it('passes_the_typed_count_to_run', () => {
    detach = attachKeys(() => ctx);
    press('7');
    press('Q', { shiftKey: true });
    expect(count).toHaveBeenLastCalledWith(7);
  });

  it('count_is_1_without_a_prefix', () => {
    detach = attachKeys(() => ctx);
    press('Q', { shiftKey: true });
    expect(count).toHaveBeenLastCalledWith(1);
  });

  it('an_open_dialog_owns_plain_keys', () => {
    detach = attachKeys(() => ctx);
    document.body.innerHTML = '<div role="dialog"></div>';
    press('W', { shiftKey: true });
    expect(plain).not.toHaveBeenCalled();
    document.body.innerHTML = '';
    press('W', { shiftKey: true });
    expect(plain).toHaveBeenCalledTimes(1);
  });
});
