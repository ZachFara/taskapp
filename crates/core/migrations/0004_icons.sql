-- Icons on the five entities. Colors on projects, cycles and views are new; statuses and labels already have
-- a color column, whose hex values are rewritten to palette keys when the database opens (palette.rs).
ALTER TABLE projects ADD COLUMN icon TEXT;
ALTER TABLE projects ADD COLUMN color TEXT;
ALTER TABLE cycles ADD COLUMN icon TEXT;
ALTER TABLE cycles ADD COLUMN color TEXT;
ALTER TABLE views ADD COLUMN icon TEXT;
ALTER TABLE views ADD COLUMN color TEXT;
ALTER TABLE labels ADD COLUMN icon TEXT;
ALTER TABLE statuses ADD COLUMN icon TEXT;
