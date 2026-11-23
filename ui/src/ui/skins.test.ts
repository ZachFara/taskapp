import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { SKINS } from '../lib/theme.svelte';

// vitest runs from ui/.
const tokens = readFileSync('src/styles/tokens.css', 'utf8');
const css = readFileSync('src/styles/skins.css', 'utf8');
const roles = [...tokens.slice(0, tokens.indexOf('\n}\n')).matchAll(/^\s*--([\w-]+):/gm)].map((m) => m[1]).filter((n) => !n.startsWith('c-'));
const lum = (hex: string) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const ratio = (a: string, b: string) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);

describe('skins.css', () => {
  it('has a block for every listed skin except the first, which is tokens.css', () => {
    expect([...css.matchAll(/^\[data-skin="(\w+)"\]\[data-skin\] \{/gm)].map((m) => m[1])).toEqual(SKINS.slice(1).map((s) => s.id));
  });
  for (const { id } of SKINS.slice(1)) {
    const body = css.slice(css.indexOf(`[data-skin="${id}"][data-skin] {`)).split('\n}\n')[0];
    const pair = (n: string, i: 0 | 1) => body.match(new RegExp(`--${n}: light-dark\\((#\\w{6}), (#\\w{6})\\)`))![i + 1];
    it(`${id} defines every role token`, () => {
      for (const r of roles) expect(body, r).toContain(`--${r}:`);
    });
    for (const [mode, i] of [['light', 0], ['dark', 1]] as const) {
      it(`${id} ${mode} meets text contrast`, () => {
        const c = (a: string, b: string, min = 4.5) => expect(ratio(pair(a, i), pair(b, i)), `${a} on ${b}`).toBeGreaterThanOrEqual(min);
        for (const bg of ['sheet', 'card', 'raised', 'active', 'selection']) c('ink', bg);
        for (const bg of ['sheet', 'card']) c('muted', bg);
        c('ink', 'band-1'); c('band-ink', 'band-2'); c('band-ink', 'band-4'); c('band-count', 'band-2');
        c('accent-ink', 'accent'); c('signal-ink', 'signal'); c('tip-ink', 'tip-bg');
        c('frame-ink', 'frame'); c('frame-muted', 'frame'); c('frame-ink', 'frame-raised'); c('frame-muted', 'frame-raised');
        for (const t of ['accent', 'stone', 'signal', 'positive', 'warning']) c(t, 'sheet', t === 'stone' || t === 'accent' ? 4.5 : 3);
        c('control', 'sheet', 2);
      });
    }
  }
});
