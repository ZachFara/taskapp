-- Sidebar folders of views. Not a foreign key: deleting a folder moves its views out in code so the change is
-- recorded and undoable. Whether a folder is open is per device and not stored.
CREATE TABLE view_folders (id INTEGER PRIMARY KEY, name TEXT NOT NULL, position INTEGER NOT NULL);
ALTER TABLE views ADD COLUMN folder_id INTEGER;
