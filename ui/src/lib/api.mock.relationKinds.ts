// Port of crates/core/src/relation_kinds.rs, so the mock keeps relation kinds exactly as the backend does.
import type { Relation, RelationKindDef } from './bindings';

type Fail = (kind: string, message: string) => never;

const seed = (id: string, name: string, forward: string, reverse: string, symmetric = false): Omit<RelationKindDef, 'position'> =>
  ({ id, name, forward, reverse, symmetric, built_in: true });
export const builtInKinds = (): RelationKindDef[] =>
  [
    seed('blocks', 'Blocks', 'blocks', 'blocked by'),
    seed('related', 'Related', 'related to', 'related to', true),
    seed('duplicate', 'Duplicate', 'duplicates', 'duplicated by'),
    seed('follows', 'Follows', 'follows', 'precedes'),
    seed('caused_by', 'Caused by', 'caused by', 'causes'),
    seed('part_of', 'Part of', 'part of', 'contains'),
  ].map((k, position) => ({ ...k, position }));

/** The id a name or an id spelled any way maps to: lower case, runs of other characters as one `_`. */
export const slug = (s: string) => s.split(/[^\p{L}\p{N}]+/u).filter(Boolean).map((w) => w.toLowerCase()).join('_');

export function relationKindsApi(kinds: RelationKindDef[], relations: Relation[], fail: Fail) {
  const sameName = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  const labels = (forward: string, reverse: string, symmetric: boolean): [string, string] => {
    const [f, r] = [forward.trim(), symmetric ? forward.trim() : reverse.trim()];
    if (!f || !r) fail('Invalid', 'a relation kind needs both labels');
    return [f, r];
  };
  const get = (id: string) => kinds.find((k) => k.id === id) ?? fail('NotFound', `relation kind ${id}`);
  return {
    /** The stored id for a kind given by id or name, in any case. */
    resolve: (kind: string) => kinds.find((k) => k.id === slug(kind))?.id ?? fail('Invalid', `unknown relation kind ${kind}`),
    list: async () => kinds,
    create: async (name: string, forward: string, reverse: string, symmetric: boolean): Promise<RelationKindDef> => {
      const [n, id] = [name.trim(), slug(name.trim())];
      if (!id) fail('Invalid', 'a relation kind needs a name');
      const [f, r] = labels(forward, reverse, symmetric);
      if (kinds.some((k) => k.id === id || sameName(k.name, n))) fail('Conflict', `relation kind ${n} already exists`);
      const made = { id, name: n, forward: f, reverse: r, symmetric, built_in: false, position: Math.max(-1, ...kinds.map((k) => k.position)) + 1 };
      kinds.push(made);
      return made;
    },
    update: async (kind: RelationKindDef): Promise<RelationKindDef> => {
      const name = kind.name.trim();
      const [f, r] = labels(kind.forward, kind.reverse, kind.symmetric);
      if (!name) fail('Invalid', 'a relation kind needs a name');
      const old = get(kind.id);
      if (old.built_in) fail('Invalid', 'a built-in relation kind cannot be changed');
      if (kinds.some((k) => k.id !== old.id && sameName(k.name, name))) fail('Conflict', `relation kind ${name} already exists`);
      Object.assign(old, { name, forward: f, reverse: r, symmetric: kind.symmetric });
      return old;
    },
    delete: async (id: string) => {
      const old = get(id);
      if (old.built_in) fail('Invalid', 'a built-in relation kind cannot be deleted');
      const used = relations.filter((x) => x.kind === id).length;
      if (used) fail('Conflict', `${used} relations use ${old.name}; remove them first`);
      kinds.splice(kinds.indexOf(old), 1);
      return null;
    },
  };
}
