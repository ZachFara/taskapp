import { describe, expect, it } from 'vitest';
import { humanDate } from './format';

describe('humanDate', () => {
  const now = new Date(2026, 10, 22);
  it('drops_the_year_when_it_is_this_year', () => expect(humanDate('2026-12-10', now)).toBe('Dec 10'));
  it('keeps_the_year_otherwise', () => expect(humanDate('2027-12-10', now)).toBe('Dec 10, 2027'));
  it('has_a_fallback_for_nothing', () => expect(humanDate(null, now, 'Set start')).toBe('Set start'));
});

describe('priority names', () => {
  it('backend 1 is Urgent, 4 is Low, and the glyph counts the other way', async () => {
    const f = (await import('./format')) as Record<string, any>;
    expect(f.PRIORITIES.slice(0, 2)).toEqual(['No priority', 'Urgent']);
    expect(f.PRIORITIES[4]).toBe('Low');
    expect([0, 1, 4].map(f.priorityGlyph)).toEqual([0, 4, 1]);
  });
});
