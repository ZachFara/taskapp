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

describe('selection columns and peek', () => {
  it('moves focus to the same row in the adjacent column, clamped to its length', () => {
    const s = createSelection();
    s.setColumns([[1, 2, 3], [4]]);
    s.focus(3);
    s.moveColumn(1);
    expect(s.focused).toBe(4);
    s.moveColumn(-1);
    expect(s.focused).toBe(1);
    s.moveColumn(-1);
    expect(s.focused).toBe(1);
  });
  it('peeks the focused row and toggles it off', () => {
    const s = createSelection();
    s.focus(2);
    s.togglePeek();
    expect(s.peeked).toBe(2);
    s.togglePeek();
    expect(s.peeked).toBe(null);
  });
});
