-- Relation kinds are data. The three original ids stay, so existing relation rows stay valid.
CREATE TABLE relation_kinds (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  forward TEXT NOT NULL, reverse TEXT NOT NULL, symmetric INTEGER NOT NULL DEFAULT 0,
  built_in INTEGER NOT NULL DEFAULT 0, position INTEGER NOT NULL);
INSERT INTO relation_kinds (id, name, forward, reverse, symmetric, built_in, position) VALUES
  ('blocks', 'Blocks', 'blocks', 'blocked by', 0, 1, 0),
  ('related', 'Related', 'related to', 'related to', 1, 1, 1),
  ('duplicate', 'Duplicate', 'duplicates', 'duplicated by', 0, 1, 2),
  ('follows', 'Follows', 'follows', 'precedes', 0, 1, 3),
  ('caused_by', 'Caused by', 'caused by', 'causes', 0, 1, 4),
  ('part_of', 'Part of', 'part of', 'contains', 0, 1, 5);

CREATE TABLE relations_new (id INTEGER PRIMARY KEY, from_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  to_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  kind TEXT NOT NULL REFERENCES relation_kinds, UNIQUE (from_id, to_id, kind));
INSERT INTO relations_new (id, from_id, to_id, kind) SELECT id, from_id, to_id, kind FROM relations;
DROP TABLE relations;
ALTER TABLE relations_new RENAME TO relations;
CREATE INDEX idx_relations_from ON relations(from_id);
CREATE INDEX idx_relations_to ON relations(to_id);
