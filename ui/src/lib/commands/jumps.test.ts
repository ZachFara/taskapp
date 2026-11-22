import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { attachKeys, type Ctx } from './index';

const press = (key: string) =>
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));

let detach: () => void;
let navigate: ReturnType<typeof vi.fn>;
beforeEach(() => {
  navigate = vi.fn();
  detach = attachKeys(() => ({ route: '/', selection: [], focusedIssueId: null, openIssueId: null, api: {} as Ctx['api'], navigate }));
});
afterEach(() => {
  detach();
  vi.useRealTimers();
});

describe('g jumps', () => {
  it.each([
    ['i', '/inbox'],
    ['m', '/my'],
    ['v', '/views'],
    ['p', '/projects'],
    ['t', '/projects/timeline'],
    ['c', '/cycles'],
    ['s', '/settings/general'],
  ])('g %s goes to %s', (key, path) => {
    press('g');
    press(key);
    expect(navigate).toHaveBeenCalledWith(path);
  });

  it('g d opens the design gallery in dev builds', () => {
    press('g');
    press('d');
    expect(navigate).toHaveBeenCalledWith('/design');
  });

  it('a pending g times out after one second', () => {
    vi.useFakeTimers();
    press('g');
    vi.advanceTimersByTime(1001);
    press('i');
    expect(navigate).not.toHaveBeenCalled();
  });
});
