-- Folders in every sidebar section, nested. Existing view folders keep their rows and become top-level folders of
-- the views section. Items carry their folder (NULL is the section's top level); a folder carries its parent.
-- New rows get a large position so they sort after items that were placed by hand (a placement renumbers from 0).
ALTER TABLE view_folders ADD COLUMN section TEXT NOT NULL DEFAULT 'view' CHECK (section IN ('pin', 'view', 'project', 'cycle'));
ALTER TABLE view_folders ADD COLUMN parent_id INTEGER;
ALTER TABLE projects ADD COLUMN folder_id INTEGER;
ALTER TABLE projects ADD COLUMN position INTEGER NOT NULL DEFAULT 1000000;
ALTER TABLE cycles ADD COLUMN folder_id INTEGER;
ALTER TABLE cycles ADD COLUMN position INTEGER NOT NULL DEFAULT 1000000;
ALTER TABLE pins ADD COLUMN folder_id INTEGER;
