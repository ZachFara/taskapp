-- A pin can point at a sidebar folder (target = the folder id). SQLite cannot change a CHECK, so the table is rebuilt.
CREATE TABLE pins_new (id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('issue','project','view','cycle','note','url','folder')),
  target TEXT NOT NULL, title TEXT, position INTEGER NOT NULL, folder_id INTEGER, UNIQUE (kind, target));
INSERT INTO pins_new (id, kind, target, title, position, folder_id) SELECT id, kind, target, title, position, folder_id FROM pins;
DROP TABLE pins;
ALTER TABLE pins_new RENAME TO pins;
