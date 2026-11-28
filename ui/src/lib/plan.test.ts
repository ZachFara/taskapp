import { describe, expect, it } from 'vitest';
import type { Issue, Status } from './bindings';
import { addDays, candidates, clock, fit, minuteAt, minutesOf, stamp } from './plan';

describe('the planning grid', () => {
  it('snaps a pointer to 15 minutes inside 07:00-22:00', () => {
    expect(minuteAt(0, 900)).toBe(7 * 60);
    expect(minuteAt(450, 900)).toBe(14 * 60 + 30);
    expect(minuteAt(5000, 900)).toBe(22 * 60 - 15);
    expect(minuteAt(-9, 900)).toBe(7 * 60);
  });
  it('keeps a block inside the column with a length of at least one step', () => {
    expect(fit(21 * 60 + 30, 120)).toEqual([20 * 60, 22 * 60]);
    expect(fit(6 * 60, 5)).toEqual([7 * 60, 7 * 60 + 15]);
    expect(fit(9 * 60 + 7, 50)).toEqual([9 * 60, 9 * 60 + 45]);
  });
  it('reads and writes local times', () => {
    expect(clock(14 * 60 + 5)).toBe('14:05');
    expect(stamp('2030-01-02', 9 * 60)).toBe('2030-01-02T09:00:00');
    expect(minutesOf('2030-01-02T14:45:00')).toBe(14 * 60 + 45);
    expect(addDays('2030-12-31', 1)).toBe('2031-01-01');
  });
});

describe('candidate issues', () => {
  const st = (id: number, name: string, category: Status['category']) => ({ id, name, category }) as Status;
  const statuses = [st(1, 'Todo', 'Unstarted'), st(2, 'Next', 'Unstarted'), st(3, 'Done', 'Completed')];
  const is = (id: number, over: Partial<Issue>) => ({ id, status_id: 1, due: null, depth: null, ...over }) as Issue;
  it('groups open issues once each and leaves finished ones out', () => {
    const issues = [
      is(1, { status_id: 2 }), is(2, { due: { Date: '2030-01-04' } }), is(3, { depth: 'deep' }),
      is(4, { status_id: 3 }), is(5, {}), is(6, { depth: 'deep', due: { Date: '2030-01-03' } }),
    ];
    const g = candidates(issues, statuses, new Set([5]), '2030-01-02', 'UTC');
    expect(g.map((x) => [x.label, x.issues.map((i) => i.id)])).toEqual([
      ['Next actions', [1, 5]], ['Due soon', [2, 6]], ['Deep work', [3]],
    ]);
  });
});
