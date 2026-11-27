-- Same text format as sqlx writes, so values sort together. Projects already carry created_at; add updated_at (last change in history, else creation). A trigger keeps it
-- current on every write; an undo that restores an older updated_at is left alone.
ALTER TABLE projects ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
UPDATE projects SET updated_at = COALESCE(
  (SELECT MAX(at) FROM changes WHERE entity IN ('project', 'row:projects') AND entity_id = CAST(projects.id AS TEXT)),
  created_at);
UPDATE projects SET updated_at = created_at WHERE updated_at < created_at;
CREATE TRIGGER projects_touch AFTER UPDATE ON projects WHEN NEW.updated_at IS OLD.updated_at BEGIN
  UPDATE projects SET updated_at = strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now') WHERE id = NEW.id;
END;
