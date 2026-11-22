import { describe, expect, it } from 'vitest';
import { area, band, line, niceTicks, scales, spread, step, summary, todayIndex, xLabels, type Day } from './geometry';

const days: Day[] = [
  { date: '2026-11-01', scope: 10, started: 2, completed: 0 },
  { date: '2026-11-02', scope: 10, started: 4, completed: 2 },
  { date: '2026-11-03', scope: 14, started: 8, completed: 4 },
  { date: '2026-11-04', scope: 14, started: 10, completed: 7 },
  { date: '2026-11-05', scope: 14, started: 12, completed: 9 },
];
const box = { w: 400, h: 200, left: 40, right: 60, top: 10, bottom: 20 };

describe('niceTicks', () => {
  it('rounds to a 1/2/5 step and the top tick sits above the maximum', () => {
    expect(niceTicks(37)).toEqual([0, 10, 20, 30, 40]);
    expect(niceTicks(8)).toEqual([0, 2, 4, 6, 8, 10]);
    expect(niceTicks(40)).toEqual([0, 10, 20, 30, 40, 50]);
  });
  it('gives an empty series a 0..1 scale', () => expect(niceTicks(0)).toEqual([0, 1]));
});

describe('scales', () => {
  const s = scales(5, 20, box);
  it('maps the first and last day to the plot edges', () => {
    expect(s.x(0)).toBe(40);
    expect(s.x(4)).toBe(340);
  });
  it('maps zero to the baseline and the top tick to the top', () => {
    expect(s.y(0)).toBe(180);
    expect(s.y(20)).toBe(10);
    expect(s.y(10)).toBe(95);
  });
  it('puts a single day at the left edge', () => expect(scales(1, 20, box).x(0)).toBe(40));
});

describe('paths', () => {
  const s = scales(5, 20, box);
  const pts = days.map((d, i) => [s.x(i), s.y(d.scope)] as [number, number]);
  it('line joins points with straight segments', () => expect(line(pts.slice(0, 2))).toBe('M40 95L115 95'));
  it('step holds a value until the next day, then rises', () => {
    expect(step(pts.slice(1, 3))).toBe('M115 95H190V61');
  });
  it('area closes down to the baseline', () => {
    expect(area(pts.slice(0, 2), 180)).toBe('M40 95L115 95L115 180L40 180Z');
  });
});

describe('todayIndex', () => {
  it('finds the day, or null when today is outside the range', () => {
    expect(todayIndex(days, '2026-11-03')).toBe(2);
    expect(todayIndex(days, '2026-12-01')).toBeNull();
    expect(todayIndex(days, undefined)).toBeNull();
  });
});

describe('xLabels', () => {
  it('uses one step that lands on the last day', () => {
    expect(xLabels(5)).toEqual([0, 1, 2, 3, 4]);
    expect(xLabels(31)).toEqual([0, 6, 12, 18, 24, 30]);
  });
  it('leaves the end unlabelled when no step lands on it', () => {
    const l = xLabels(14);
    expect(l[0]).toBe(0);
    expect(l.at(-1)).not.toBe(13);
  });
  it('handles one day', () => expect(xLabels(1)).toEqual([0]));
});

describe('summary', () => {
  it('states the range and the last day in words', () => {
    expect(summary(days, 'points')).toBe('Progress in points from 2026-11-01 to 2026-11-05. On the last day: scope 14, started 12, completed 9 (64%).');
  });
  it('says so when there is nothing', () => expect(summary([], 'count')).toBe('No progress data yet.'));
});

describe('band and spread', () => {
  it('band runs along the upper line and back along the lower', () => {
    expect(band([[0, 1], [10, 2]], [[0, 5], [10, 6]])).toBe('M0 1L10 2L10 6L0 5Z');
  });
  it('spread keeps labels a gap apart in their order', () => {
    expect(spread([50, 52, 100], 13)).toEqual([50, 63, 100]);
    expect(spread([30], 13)).toEqual([30]);
  });
});
