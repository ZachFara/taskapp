import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import Kbd from './Kbd.svelte';

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const caps = (keys: string | string[]) => {
  app = mount(Kbd, { target: document.body, props: { keys } });
  return [...document.querySelectorAll('kbd')].map((k) => k.textContent);
};

describe('Kbd glyphs', () => {
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
