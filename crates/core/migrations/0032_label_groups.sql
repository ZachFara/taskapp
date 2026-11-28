-- Label groups: a named set of labels (a label is in at most one). An exclusive group lets an issue hold one of its labels.
CREATE TABLE label_groups (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, exclusive INTEGER NOT NULL DEFAULT 0 CHECK (exclusive IN (0, 1)));
ALTER TABLE labels ADD COLUMN group_id INTEGER REFERENCES label_groups ON DELETE SET NULL;
