-- Projects get their own statuses, a priority, labels and custom property values, like issues.
CREATE TABLE project_statuses (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  category TEXT NOT NULL CHECK (category IN ('planned','started','paused','completed','cancelled')),
  position INTEGER NOT NULL, color TEXT NOT NULL, icon TEXT);
INSERT INTO project_statuses (name, category, position, color) VALUES
  ('Planned', 'planned', 0, 'gray'), ('In progress', 'started', 1, 'amber'), ('Paused', 'paused', 2, 'orange'),
  ('Done', 'completed', 3, 'indigo'), ('Cancelled', 'cancelled', 4, 'gray');

-- Nullable because SQLite cannot add a NOT NULL reference column; the trigger keeps it set on every insert.
ALTER TABLE projects ADD COLUMN status_id INTEGER REFERENCES project_statuses;
UPDATE projects SET status_id = 2;
CREATE TRIGGER projects_default_status AFTER INSERT ON projects WHEN NEW.status_id IS NULL
BEGIN
  UPDATE projects SET status_id = (SELECT id FROM project_statuses ORDER BY position, id LIMIT 1) WHERE id = NEW.id;
END;
ALTER TABLE projects ADD COLUMN priority INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 4);
ALTER TABLE projects ADD COLUMN properties TEXT NOT NULL DEFAULT '{}';

-- Own id so row snapshots (undo) have a single primary key.
CREATE TABLE project_labels (id INTEGER PRIMARY KEY, project_id INTEGER NOT NULL REFERENCES projects ON DELETE CASCADE,
  label_id INTEGER NOT NULL REFERENCES labels ON DELETE CASCADE, UNIQUE (project_id, label_id));
CREATE INDEX idx_project_labels_label ON project_labels(label_id);

-- Which entities a custom property applies to. `project_id` still limits the issue side to one project.
ALTER TABLE property_defs ADD COLUMN applies_to TEXT NOT NULL DEFAULT 'issues' CHECK (applies_to IN ('issues','projects','both'));
