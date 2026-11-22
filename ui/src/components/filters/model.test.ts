import { describe, expect, it } from 'vitest';
import type { Filter } from '../../lib/bindings';
import { buildFilter, opsFor, splitFilter, type Chip } from './model';

const cond = (op: Chip['op'], value: Chip['value'], fixed: 'Status' | 'Title' = 'Status'): Chip => ({
  id: 1, field: { Fixed: fixed }, op, value,
});

describe('filter model', () => {
  it('operators_match_field_type', () => {
    expect(opsFor('Date')).toContain('Lt');
    expect(opsFor('Text')).not.toContain('Lt');
    expect(opsFor('Select')).toContain('In');
  });

  it('incomplete_chips_are_not_saved', () => {
    expect(buildFilter(null, [cond('In', [])])).toBeNull();
    expect(buildFilter(null, [cond('Contains', ''), cond('IsEmpty', 'x')])).toEqual({
      Cond: { field: { Fixed: 'Status' }, op: 'IsEmpty', value: null },
    });
  });

  it('flat_and_round_trips_through_chips', () => {
    const f: Filter = { And: [
      { Cond: { field: { Fixed: 'Status' }, op: 'In', value: [1, 2] } },
      { Cond: { field: { Fixed: 'Title' }, op: 'Contains', value: 'x' } },
    ] };
    const { chips, advanced } = splitFilter(f);
    expect(advanced).toBeNull();
    expect(chips.map((c) => c.op)).toEqual(['In', 'Contains']);
    expect(buildFilter(advanced, chips)).toEqual(f);
  });

  it('nested_filters_are_kept_whole_as_advanced', () => {
    const f: Filter = { Or: [{ Cond: { field: { Fixed: 'Title' }, op: 'Eq', value: 'a' } }] };
    const { chips, advanced } = splitFilter(f);
    expect(chips).toEqual([]);
    expect(advanced).toEqual(f);
    expect(buildFilter(advanced, [cond('In', [3])])).toEqual({
      And: [f, { Cond: { field: { Fixed: 'Status' }, op: 'In', value: [3] } }],
    });
  });
});
