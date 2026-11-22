-- A template is a saved view definition new views start from. A separate table (not a flag on views) keeps
-- templates out of every view listing, pin and sidebar without a filter at each reader.
CREATE TABLE view_templates (id INTEGER PRIMARY KEY, name TEXT NOT NULL, definition TEXT NOT NULL,
  position INTEGER NOT NULL, icon TEXT, color TEXT);
-- Not a foreign key: deleting a template clears this in code so the change is recorded and undoable.
ALTER TABLE views ADD COLUMN template_id INTEGER;
