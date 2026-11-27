import { describe, expect, it } from 'vitest';
import { combineFilters } from './ephemeral';

const cond = (value: string) => ({ Cond: { field: { Fixed: 'Title' as const }, op: 'Contains' as const, value } });

describe('combineFilters', () => {
  it('is the saved filter alone when there is no quick filter, and null when there is neither', () => {
    expect(combineFilters(null, null)).toBeNull();
    expect(combineFilters(cond('a'), null)).toEqual(cond('a'));
    expect(combineFilters(null, cond('b'))).toEqual(cond('b'));
  });

  it('requires both when both are set', () => {
    expect(combineFilters(cond('a'), cond('b'))).toEqual({ And: [cond('a'), cond('b')] });
  });
});
