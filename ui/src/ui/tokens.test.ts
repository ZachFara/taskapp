import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// vitest runs from ui/.
const css = readFileSync('src/styles/tokens.css', 'utf8');
const rust = readFileSync('../crates/core/src/palette.rs', 'utf8');
const block = (open: string, close: string) => {
  const from = css.indexOf(open) + open.length;
  return css.slice(from, css.indexOf(close, from));
};
const names = (b: string) => [...b.matchAll(/^\s*(--[\w-]+):/gm)].map((m) => m[1]);
const dec = (b: string) => b.split('\n').map((l) => l.trim()).filter(Boolean);

describe('tokens.css', () => {
  const light = block(':root,\n[data-theme="light"] {', '\n}\n');
  const dark = block('[data-theme="dark"] {', '\n}\n');
  const auto = block(':root:not([data-theme]) {', '\n  }\n');

  it('defines the same role tokens in every theme block', () => {
    expect(names(dark)).toEqual(names(light));
    expect(names(auto)).toEqual(names(light));
  });
  it('keeps the explicit dark block and the system-dark block identical', () => {
    expect(dec(auto)).toEqual(dec(dark));
  });
  it('has a colour for every palette key the backend knows, in both themes', () => {
    const keys = [...rust.matchAll(/\("(\w+)", "#[0-9a-f]{6}"\)/g)].map((m) => m[1]);
    expect(keys).toHaveLength(12);
    for (const k of keys) {
      expect(light).toContain(`--c-${k}:`);
      expect(dark).toContain(`--c-${k}:`);
    }
  });
  it('gives the palette 3:1 contrast on the sheet and band steps 1 and 2 in both themes', () => {
    const lum = (hex: string) => {
      const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const ratio = (a: string, b: string) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
    for (const theme of [light, dark]) {
      const v = (n: string) => theme.match(new RegExp(`${n}: (#[0-9a-f]{6})`))![1];
      for (const bg of ['sheet', 'band-1', 'band-2']) {
        for (const [, name, hex] of theme.matchAll(/--(c-\w+): (#[0-9a-f]{6})/g)) {
          expect(ratio(hex, v(`--${bg}`)), `${name} on ${bg}`).toBeGreaterThanOrEqual(3);
        }
      }
    }
  });
  it('uses the three motion durations, each scaled by the animation speed', () => {
    expect(css).toMatch(/--motion-scale: 1;/);
    expect(css).toMatch(/--dur-1: calc\(80ms \* var\(--motion-scale\)\)/);
    expect(css).toMatch(/--dur-2: calc\(140ms \* var\(--motion-scale\)\)/);
    expect(css).toMatch(/--dur-3: calc\(220ms \* var\(--motion-scale\)\)/);
  });
});
