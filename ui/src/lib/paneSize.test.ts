import { afterEach, describe, expect, it, vi } from 'vitest';
import { clampWidth, readWidth, writeWidth } from './paneSize';

afterEach(() => vi.restoreAllMocks());

describe('paneSize', () => {
  it('clamps a width to its bounds', () => {
    expect(clampWidth(10, 180, 360)).toBe(180);
    expect(clampWidth(900, 180, 360)).toBe(360);
    expect(clampWidth(250.4, 180, 360)).toBe(250);
  });

  it('remembers a width and forgets it on null', () => {
    writeWidth('cairn.test.w', 260);
    expect(readWidth('cairn.test.w')).toBe(260);
    writeWidth('cairn.test.w', null);
    expect(readWidth('cairn.test.w')).toBeNull();
  });

  it('reads nothing from garbage or from storage that throws', () => {
    localStorage.setItem('cairn.test.bad', 'wide');
    expect(readWidth('cairn.test.bad')).toBeNull();
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(readWidth('cairn.test.w')).toBeNull();
  });

  it('does not throw when storage is blocked for writes', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(() => writeWidth('cairn.test.w', 300)).not.toThrow();
  });
});
