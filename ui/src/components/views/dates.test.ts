import { describe, expect, it } from 'vitest';
import type { Issue } from '../../lib/bindings';
import { dueDay, isOverdue } from './dates';

const due = (utc: string, tz: string) => ({ due: { DateTime: { utc, tz } } }) as Pick<Issue, 'due'>;

describe('workspace-day dates', () => {
  it('dueDay_uses_the_local_day_of_the_zone', () => {
    // 02:00Z on Dec 1 is still Nov 30 in New York and already Dec 1 in Auckland.
    expect(dueDay(due('2026-12-01T02:00:00Z', 'US/Eastern'), 'US/Eastern')).toBe('2026-11-30');
    expect(dueDay(due('2026-12-01T02:00:00Z', 'Pacific/Auckland'), 'Pacific/Auckland')).toBe('2026-12-01');
    expect(dueDay({ due: { Date: '2026-12-01' } }, 'US/Eastern')).toBe('2026-12-01');
    expect(dueDay({ due: null }, 'UTC')).toBeNull();
  });

  it('isOverdue_flips_at_local_midnight_not_utc_midnight', () => {
    const now = new Date('2026-12-01T03:00:00Z'); // Nov 30 evening in New York, Dec 1 in Auckland
    const i = { due: { Date: '2026-11-30' } };
    expect(isOverdue(i, 'Completed', 'US/Eastern', now)).toBe(false);
    expect(isOverdue(i, 'Started', 'US/Eastern', now)).toBe(false);
    expect(isOverdue(i, 'Started', 'Pacific/Auckland', now)).toBe(true);
    expect(isOverdue(i, 'Completed', 'Pacific/Auckland', now)).toBe(false);
  });
});
