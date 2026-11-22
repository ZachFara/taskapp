import { describe, expect, it } from 'vitest';
import { humanDate } from './format';

describe('humanDate', () => {
  const now = new Date(2026, 10, 22);
  it('drops_the_year_when_it_is_this_year', () => expect(humanDate('2026-12-10', now)).toBe('Dec 10'));
  it('keeps_the_year_otherwise', () => expect(humanDate('2027-12-10', now)).toBe('Dec 10, 2027'));
  it('has_a_fallback_for_nothing', () => expect(humanDate(null, now, 'Set start')).toBe('Set start'));
});
