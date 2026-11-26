// Port of crates/core/src/view_folders.rs, so the mock nests, places and deletes folders exactly as the backend does.
import type { FolderSection, Placement, SavedView, ViewFolder, ViewFolderId } from './bindings';

type Fail = (kind: string, message: string) => never;

export const MAX_DEPTH = 3;
/** The position a row has until it is placed by hand; a placement renumbers its siblings from 0. */
const UNPLACED = 1_000_000;
const SECTIONS: FolderSection[] = ['pin', 'view', 'project', 'cycle'];

/**
 * `placed` holds the pins, projects and cycles that were ever placed (views keep their place on the view);
 * `ids` lists the ids that exist in a section, in their natural order; `start` is where an unplaced row sits
 * (a pin has its own position from the start) and `moved` hears a row's new position (a pin keeps it as its own).
 */
export function foldersApi(
  folders: ViewFolder[],
  placed: Placement[],
  views: SavedView[],
  ids: (s: FolderSection) => number[],
  start: (s: FolderSection, id: number) => number,
  moved: (s: FolderSection, id: number, position: number) => void,
  fail: Fail,
) {
  const get = (id: number) => folders.find((f) => f.id === id) ?? fail('NotFound', `folder ${id}`);
  const rowOf = (s: FolderSection, id: number) => (s === 'view' ? views.find((v) => v.id === id) : placed.find((p) => p.section === s && p.id === id));
  const above = (id: number) => {
    const out: number[] = [];
    for (let at = get(id).parent_id; at !== null && !out.includes(at); at = get(at).parent_id) out.push(at);
    return out;
  };
  const height = (id: number): number => 1 + Math.max(0, ...folders.filter((f) => f.parent_id === id).map((f) => height(f.id)));
  const target = (s: FolderSection, folder: ViewFolderId | null) => {
    if (folder === null) return null;
    const f = get(folder);
    if (f.section !== s) fail('Invalid', 'a folder only holds items of its own section');
    return f;
  };
  const tooDeep = () => fail('Invalid', `folders nest at most ${MAX_DEPTH} deep`);
  // The ids in `order` get positions 0.. in that order.
  const number = (order: number[], write: (id: number, position: number) => void) => order.forEach((id, i) => write(id, i));
  const inserted = (siblings: number[], id: number, index: number | null) => {
    const rest = siblings.filter((x) => x !== id);
    rest.splice(Math.min(index ?? rest.length, rest.length), 0, id);
    return rest;
  };
  const of = (s: FolderSection, folder: number | null) =>
    ids(s)
      .filter((id) => (rowOf(s, id)?.folder_id ?? null) === folder)
      .sort((a, b) => (rowOf(s, a)?.position ?? start(s, a)) - (rowOf(s, b)?.position ?? start(s, b)));
  return {
    list: async () => [...folders].sort((a, b) => a.position - b.position || a.id - b.id),
    items: async (): Promise<Placement[]> =>
      SECTIONS.flatMap((s) =>
        ids(s)
          .map((id) => ({ section: s, id, folder_id: rowOf(s, id)?.folder_id ?? null, position: rowOf(s, id)?.position ?? start(s, id) }))
          .sort((a, b) => a.position - b.position || a.id - b.id),
      ),
    create: async (section: FolderSection, parent: ViewFolderId | null, name: string) => {
      const clean = name.trim() || fail('Invalid', 'a folder needs a name');
      const p = target(section, parent);
      if (p && above(p.id).length + 2 > MAX_DEPTH) tooDeep();
      const f: ViewFolder = { id: Math.max(0, ...folders.map((x) => x.id)) + 1, name: clean, position: Math.max(-1, ...folders.map((x) => x.position)) + 1, section, parent_id: parent };
      folders.push(f);
      return f;
    },
    rename: async (id: number, name: string) => Object.assign(get(id), { name: name.trim() || fail('Invalid', 'a folder needs a name') }),
    delete: async (id: number) => {
      const f = get(id);
      for (const x of folders) if (x.parent_id === id) x.parent_id = f.parent_id;
      for (const s of SECTIONS) for (const id2 of ids(s)) {
        const r = rowOf(s, id2);
        if (r?.folder_id === id) r.folder_id = f.parent_id;
      }
      folders.splice(folders.indexOf(f), 1);
      return null;
    },
    place: async (id: number, parent: ViewFolderId | null, index: number | null) => {
      const f = get(id);
      const p = target(f.section, parent);
      if (p) {
        const up = above(p.id);
        if (p.id === id || up.includes(id)) fail('Invalid', 'a folder cannot go inside itself');
        if (up.length + 1 + height(id) > MAX_DEPTH) tooDeep();
      }
      const siblings = [...folders].sort((a, b) => a.position - b.position || a.id - b.id).filter((x) => x.section === f.section && x.parent_id === parent).map((x) => x.id);
      f.parent_id = parent;
      number(inserted(siblings, id, index), (x, position) => (get(x).position = position));
      return null;
    },
    placeItem: async (section: FolderSection, id: number, folder: ViewFolderId | null, index: number | null) => {
      target(section, folder);
      if (!ids(section).includes(id)) fail('NotFound', `${section} ${id}`);
      let row = rowOf(section, id);
      const siblings = of(section, folder);
      if (!row) placed.push((row = { section, id, folder_id: null, position: UNPLACED }));
      row.folder_id = folder;
      number(inserted(siblings, id, index), (x, position) => {
        const r = rowOf(section, x);
        if (r) r.position = position;
        else placed.push({ section, id: x, folder_id: folder, position });
        moved(section, x, position);
      });
      if (section === 'view') views.sort((a, b) => a.position - b.position || a.id - b.id);
      return null;
    },
  };
}
