import { mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { keycaps } from '../lib/commands/glyphs';
import Kbd from './Kbd.svelte';

let app: ReturnType<typeof mount> | null = null;
beforeEach(() => void vi.spyOn(navigator, 'platform', 'get').mockReturnValue(MAC));
afterEach(() => {
  vi.restoreAllMocks();
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const caps = (keys: string | string[]) => {
  app = mount(Kbd, { target: document.body, props: { keys } });
  return [...document.querySelectorAll('kbd')].map((k) => k.textContent);
};

const MAC = 'MacIntel';
describe('Kbd glyphs', () => {
  it('non-mac platforms spell the modifiers out', () => {
    expect(keycaps('mod+shift+z', 'Linux x86_64')).toEqual(['Ctrl+Shift+Z']);
    expect(keycaps('alt+x', 'Win32')).toEqual(['Alt+X']);
    expect(keycaps('g i', 'Linux x86_64')).toEqual(['G', 'I']);
  });

  it.each([
    ['mod+shift+z', ['⌘⇧Z']],
    ['mod+d', ['⌘D']],
    ['alt+x', ['⌥X']],
    ['f2', ['F2']],
    ['arrowdown', ['↓']],
    ['escape', ['Esc']],
    ['enter', ['↵']],
    ['shift+enter', ['⇧↵']],
    ['shift+g', ['⇧G']],
    ['G', ['G']],
    ['j', ['J']],
    ['?', ['?']],
    ['⌘K', ['⌘K']],
    ['Esc', ['Esc']],
  ])('%s renders as %j', (key, want) => expect(caps(key)).toEqual(want));

  it('a sequence is one keycap per key', () => expect(caps('g i')).toEqual(['G', 'I']));
  it('a list of keys keeps one keycap each', () => expect(caps(['mod+k', 'escape'])).toEqual(['⌘K', 'Esc']));
});
