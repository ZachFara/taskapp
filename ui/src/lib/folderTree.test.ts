import { describe, expect, it } from 'vitest';
import type { Placement, ViewFolder } from './bindings';
import { buildRows, canNest, choices } from './folderTree';

const f = (id: number, name: string, parent_id: number | null = null, section: ViewFolder['section'] = 'view'): ViewFolder => ({ id, name, position: id, section, parent_id });
const at = (id: number, folder_id: number | null, position = 0): Placement => ({ section: 'view', id, folder_id, position });

describe('folder tree', () => {
  const folders = [f(1, 'A'), f(2, 'B', 1), f(3, 'C', 2), f(4, 'Loose'), f(5, 'P', null, 'project')];

  it('nests at most three deep, never into itself or its own inside, never across sections', () => {
    expect(canNest(folders, 4, 3)).toBe(false); // A > B > C > Loose is four
    expect(canNest(folders, 3, 4)).toBe(true);
    expect(canNest(folders, 1, 3)).toBe(false);
    expect(canNest(folders, 1, 1)).toBe(false);
    expect(canNest(folders, 4, 5)).toBe(false);
    expect(canNest(folders, 3, null)).toBe(true);
  });

  it('lists the choices in tree order with their depth, leaving out the folder own subtree', () => {
    expect(choices(folders, 'view').map((c) => `${c.depth}${c.name}`)).toEqual(['0A', '1B', '2C', '0Loose']);
    expect(choices(folders, 'view', 2).map((c) => c.name)).toEqual(['A', 'Loose']);
  });

  it('builds rows: folders first, nested, then the items of each folder', () => {
    const items = [{ id: 10 }, { id: 11 }, { id: 12 }];
    const rows = buildRows('view', folders, [at(10, 3), at(11, null, 2), at(12, null, 1)], items, { placed: true });
    expect(rows.map((r) => (r.type === 'folder' ? `F${r.folder.name}@${r.depth}` : `i${r.item.id}@${r.depth}`))).toEqual(['FA@0', 'FB@1', 'FC@2', 'i10@3', 'FLoose@0', 'i12@0', 'i11@0']);
    const shut = buildRows('view', folders, [at(10, 3)], items, { shut: { 2: true } });
    expect(shut.filter((r) => r.type === 'item').map((r) => (r as { item: { id: number } }).item.id)).toEqual([11, 12]);
    expect(buildRows('view', folders, [at(10, 3)], items, { hideEmpty: true }).filter((r) => r.type === 'folder').length).toBe(3);
  });
});
