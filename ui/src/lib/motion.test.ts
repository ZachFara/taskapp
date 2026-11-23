import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { motion, readSpeed, setSpeed, stepSpeed, SPEEDS } from './motion.svelte';

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
