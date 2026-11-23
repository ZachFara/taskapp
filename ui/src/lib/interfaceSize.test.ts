import { afterEach, describe, expect, it, vi } from 'vitest';
import { interfaceSize, readSize, setInterfaceSize, stepInterfaceSize } from './interfaceSize.svelte';

afterEach(() => {
  setInterfaceSize(110);
  vi.restoreAllMocks();
});

describe('interfaceSize', () => {
  it('steps through 100, 110, 115 and 125 and stops at both ends', () => {
    setInterfaceSize(110);
    stepInterfaceSize(1);
    expect(interfaceSize.size).toBe(115);
    stepInterfaceSize(1);
    stepInterfaceSize(1);
    expect(interfaceSize.size).toBe(125);
    setInterfaceSize(100);
    stepInterfaceSize(-1);
    expect(interfaceSize.size).toBe(100);
  });

  it('publishes the zoom factor on the root and remembers it', () => {
    setInterfaceSize(125);
    expect(document.documentElement.style.getPropertyValue('--ui-zoom')).toBe('1.25');
    expect(localStorage.getItem('cairn.interfaceSize')).toBe('125');
  });

  it('falls back to the default 110 for a stored value it does not know', () => {
    localStorage.setItem('cairn.interfaceSize', '300');
    expect(readSize()).toBe(110);
    localStorage.setItem('cairn.interfaceSize', '125');
    expect(readSize()).toBe(125);
  });
});
