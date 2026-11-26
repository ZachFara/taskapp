import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flash, fresh, motion, ms, readSpeed, replay, setSpeed, stepSpeed, SPEEDS } from './motion.svelte';

const root = () => document.documentElement;
const reduce = (on: boolean) =>
  vi.stubGlobal('matchMedia', (q: string) => ({ matches: on && q.includes('reduce'), media: q, addEventListener() {}, removeEventListener() {} }));

beforeEach(() => {
  localStorage.clear();
  reduce(false);
});
afterEach(() => {
  setSpeed('normal');
  localStorage.clear();
  vi.unstubAllGlobals();
});

describe('animation speed', () => {
  it('defaults to Normal and applies scale 1', () => {
    expect(readSpeed()).toBe('normal');
    expect(root().style.getPropertyValue('--motion-scale')).toBe('1');
  });

  it('Off sets the scale to 0 and remembers the choice', () => {
    setSpeed('off');
    expect(root().style.getPropertyValue('--motion-scale')).toBe('0');
    expect(localStorage.getItem('cairn.animationSpeed')).toBe('off');
    expect(root().hasAttribute('data-motion-set')).toBe(true);
  });

  it('Fast and Slow scale the durations', () => {
    setSpeed('fast');
    expect(root().style.getPropertyValue('--motion-scale')).toBe(String(SPEEDS.fast));
    setSpeed('slow');
    expect(root().style.getPropertyValue('--motion-scale')).toBe(String(SPEEDS.slow));
  });

  it('steps slower and faster and holds at both ends', () => {
    setSpeed('normal');
    stepSpeed(-1);
    expect(motion.speed).toBe('fast');
    stepSpeed(-1);
    stepSpeed(-1);
    expect(motion.speed).toBe('off');
    setSpeed('slow');
    stepSpeed(1);
    expect(motion.speed).toBe('slow');
  });

  it('forces Off under reduced motion unless the user picked a speed', () => {
    reduce(true);
    expect(readSpeed()).toBe('off');
    setSpeed('slow');
    expect(readSpeed()).toBe('slow');
  });

  it('falls back to Normal for a stored value it does not know', () => {
    localStorage.setItem('cairn.animationSpeed', 'warp');
    expect(readSpeed()).toBe('normal');
  });
});

describe('motion helpers', () => {
  // jsdom does not resolve var() or calc(), so the probe's computed duration is stood in by scale * base.
  const resolve = (base: number) =>
    vi.spyOn(window, 'getComputedStyle').mockImplementation(() => ({ animationDuration: `${base * SPEEDS[motion.speed]}ms` }) as CSSStyleDeclaration);

  it('ms reads a duration token, and is 0 when animation is Off', () => {
    resolve(120);
    expect(ms('--dur-out')).toBe(120);
    setSpeed('off');
    expect(ms('--dur-out')).toBe(0);
    vi.restoreAllMocks();
  });

  it('ms reads seconds too', () => {
    vi.spyOn(window, 'getComputedStyle').mockImplementation(() => ({ animationDuration: '0.22s' }) as CSSStyleDeclaration);
    expect(ms('--dur-3')).toBeCloseTo(220);
    vi.restoreAllMocks();
  });

  it('replay restarts the class', () => {
    const el = document.createElement('div');
    el.className = 'cx-page';
    replay(el, 'cx-page');
    expect(el.classList.contains('cx-page')).toBe(true);
  });

  it('flash plays for the fresh issue once, and when the same issue changes, not when the row shows another issue', () => {
    const el = document.createElement('div');
    fresh.key = 'ABC-1';
    const a = flash(el, { id: 1, sig: '1:0', key: 'ABC-1' });
    expect(el.classList.contains('cx-flash')).toBe(true);
    expect(fresh.key).toBeNull();
    el.classList.remove('cx-flash');
    a.update({ id: 2, sig: '1:0' }); // a recycled row
    expect(el.classList.contains('cx-flash')).toBe(false);
    a.update({ id: 2, sig: '2:0' }); // same issue, new status
    expect(el.classList.contains('cx-flash')).toBe(true);
  });
});
