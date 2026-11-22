import { describe, expect, it } from 'vitest';
import { createSelection } from './selection.svelte';

const ids = [10, 11, 12, 13];
describe('selection', () => {
  it('moves focus and clamps at the ends', () => {
    const s = createSelection();
    s.move(ids, 1);
    expect(s.focused).toBe(10);
    s.move(ids, 9);
    expect(s.focused).toBe(13);
    s.move(ids, -1);
    expect(s.focused).toBe(12);
  });
  it('extends a range both ways from the anchor', () => {
    const s = createSelection();
    s.focus(11);
    s.move(ids, 1, true);
    s.move(ids, 1, true);
    expect(s.selected).toEqual([11, 12, 13]);
    s.move(ids, -1, true);
    expect(s.selected).toEqual([11, 12]);
  });
  it('toggles the focused row, selects all and clears', () => {
    const s = createSelection();
    s.focus(12);
    s.toggle();
    expect(s.selected).toEqual([12]);
    s.toggle();
    expect(s.selected).toEqual([]);
    s.selectAll(ids);
    expect(s.selected).toEqual(ids);
    s.clear();
    expect(s.selected).toEqual([]);
  });
});
