import { describe, expect, it } from 'vitest';
import { bucketFor, burnupDays, filterOf, rangeOf, type Metrics } from './metrics';

const cycles = [{ id: 7, start_date: '2026-11-23', end_date: '2026-12-06' }];
const ctx = { today: '2026-11-27', weekStart: 'monday' as const, cycles };

describe('rangeOf', () => {
  it('Today is one day', () => expect(rangeOf('today', ctx)).toMatchObject({ from: '2026-11-27', to: '2026-11-27' }));
  it('This week runs from the week start for seven days', () => {
    expect(rangeOf('week', ctx)).toMatchObject({ from: '2026-11-23', to: '2026-11-29' });
    expect(rangeOf('week', { ...ctx, weekStart: 'sunday' })).toMatchObject({ from: '2026-11-22', to: '2026-11-28' });
  });
  it('Month, Quarter and Year cover the calendar span', () => {
    expect(rangeOf('month', ctx)).toMatchObject({ from: '2026-11-01', to: '2026-11-30' });
    expect(rangeOf('quarter', ctx)).toMatchObject({ from: '2026-10-01', to: '2026-12-31' });
    expect(rangeOf('year', ctx)).toMatchObject({ from: '2026-01-01', to: '2026-12-31' });
  });
  it('a cycle is its own scope and dates; "cycle" is the one holding today', () => {
    expect(rangeOf('cycle', ctx)).toMatchObject({ scope: { Cycle: 7 }, from: '2026-11-23', to: '2026-12-06' });
    expect(rangeOf('cycle:7', ctx)?.scope).toEqual({ Cycle: 7 });
    expect(rangeOf('cycle:9', ctx)).toBeNull();
    expect(rangeOf('cycle', { ...ctx, today: '2027-03-01' })).toBeNull();
  });
  it('Custom uses the two dates, in order', () => {
    expect(rangeOf('custom', ctx, '2026-11-10', '2026-11-02')).toMatchObject({ from: '2026-11-02', to: '2026-11-10', scope: { Range: { from: '2026-11-02', to: '2026-11-10' } } });
    expect(rangeOf('custom', ctx, '', '2026-11-02')).toBeNull();
  });
});

describe('bucketFor', () => {
  it('picks Day up to 31 days, Week up to about 6 months, Month beyond', () => {
    expect(bucketFor('2026-11-01', '2026-12-01')).toBe('Day');
    expect(bucketFor('2026-11-01', '2026-12-02')).toBe('Week');
    expect(bucketFor('2026-01-01', '2026-06-30')).toBe('Week');
    expect(bucketFor('2026-01-01', '2026-12-31')).toBe('Month');
  });
});

describe('filterOf', () => {
  it('is null with nothing chosen, one condition, or both together', () => {
    expect(filterOf(null, null)).toBeNull();
    expect(filterOf(2, null)).toEqual({ Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: 2 } });
    expect(filterOf(2, 5)).toEqual({ And: [{ Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: 2 } }, { Cond: { field: { Fixed: 'Label' }, op: 'Eq', value: 5 } }] });
  });
});

describe('burnupDays', () => {
  it('turns buckets into scope and done lines', () => {
    const m = { buckets: [{ start: '2026-11-01', scope: 4, done_total: 1 }, { start: '2026-11-02', scope: 5, done_total: 3 }] } as Metrics;
    expect(burnupDays(m)).toEqual([
      { date: '2026-11-01', scope: 4, started: 1, completed: 1 },
      { date: '2026-11-02', scope: 5, started: 3, completed: 3 },
    ]);
  });
});
