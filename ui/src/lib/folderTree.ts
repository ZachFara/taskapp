// The folder rules of crates/core/src/view_folders.rs, and the rows a section shows: folders first (nested), then the
// items placed directly in each folder.
import type { FolderSection, Placement, ViewFolder } from './bindings';

export const MAX_DEPTH = 3;
/** The position of an item nobody placed by hand, as the backend stores it. */
export const UNPLACED = 1_000_000;

export type FolderRow = { type: 'folder'; folder: ViewFolder; depth: number; shut: boolean; count: number; pos: number; container: number | null };
export type ItemRow<T> = { type: 'item'; item: T; depth: number; pos: number; container: number | null };
export type TreeRow<T> = FolderRow | ItemRow<T>;

const byPosition = (a: ViewFolder, b: ViewFolder) => a.position - b.position || a.id - b.id;

/** Folders above `id`, nearest first. */
export function above(folders: ViewFolder[], id: number): number[] {
  const out: number[] = [];
  for (let at = folders.find((f) => f.id === id)?.parent_id ?? null; at !== null && !out.includes(at); at = folders.find((f) => f.id === at)?.parent_id ?? null) out.push(at);
  return out;
}

/** 1 for a folder with no folders in it, plus the deepest chain below. */
export const height = (folders: ViewFolder[], id: number): number =>
  1 + Math.max(0, ...folders.filter((f) => f.parent_id === id).map((f) => height(folders, f.id)));

/** Whether `id` may go inside `parent` (null: the top level): same section, not into itself, within MAX_DEPTH. */
export function canNest(folders: ViewFolder[], id: number, parent: number | null): boolean {
  if (parent === null) return true;
  const [f, p] = [folders.find((x) => x.id === id), folders.find((x) => x.id === parent)];
  if (!f || !p || f.section !== p.section || p.id === id) return false;
  const up = above(folders, p.id);
  return !up.includes(id) && up.length + 1 + height(folders, id) <= MAX_DEPTH;
}

/**
 * The folders of `section` in tree order with their depth (0 for top level), for a "Move to folder" list. With `moving`,
 * only the places that folder may go.
 */
export function choices(folders: ViewFolder[], section: FolderSection, moving?: number): { id: number; name: string; depth: number }[] {
  const walk = (parent: number | null, depth: number): { id: number; name: string; depth: number }[] =>
    folders
      .filter((f) => f.section === section && f.parent_id === parent)
      .sort(byPosition)
      .flatMap((f) => [{ id: f.id, name: f.name, depth }, ...walk(f.id, depth + 1)]);
  return walk(null, 0).filter((c) => moving === undefined || canNest(folders, moving, c.id));
}

/**
 * Every row of one section, flat. Items inside a shut folder are left out. `items` come in the order the caller wants;
 * with `placed` they are re-ordered by their place in the folder (a page with its own sort passes none). `hideEmpty`
 * drops folders that hold no item at any depth.
 */
export function buildRows<T extends { id: number }>(
  section: FolderSection,
  folders: ViewFolder[],
  places: Placement[],
  items: T[],
  opts: { shut?: Record<number, boolean>; placed?: boolean; hideEmpty?: boolean } = {},
): TreeRow<T>[] {
  const where = new Map(places.filter((p) => p.section === section).map((p) => [p.id, p]));
  const inside = (c: number | null) => {
    const list = items.filter((i) => (where.get(i.id)?.folder_id ?? null) === c);
    return opts.placed ? list.sort((a, b) => (where.get(a.id)?.position ?? UNPLACED) - (where.get(b.id)?.position ?? UNPLACED)) : list;
  };
  const total = (id: number): number => inside(id).length + folders.filter((f) => f.section === section && f.parent_id === id).reduce((n, f) => n + total(f.id), 0);
  const walk = (container: number | null, depth: number): TreeRow<T>[] => {
    const kids = folders.filter((f) => f.section === section && f.parent_id === container).sort(byPosition).filter((f) => !opts.hideEmpty || total(f.id) > 0);
    const out: TreeRow<T>[] = kids.flatMap((folder, pos): TreeRow<T>[] => {
      const shut = !!opts.shut?.[folder.id];
      return [{ type: 'folder', folder, depth, shut, count: total(folder.id), pos, container }, ...(shut ? [] : walk(folder.id, depth + 1))];
    });
    return [...out, ...inside(container).map((item, pos): TreeRow<T> => ({ type: 'item', item, depth, pos, container }))];
  };
  return walk(null, 0);
}
