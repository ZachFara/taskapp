CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); -- JSON values
CREATE TABLE statuses (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  category TEXT NOT NULL CHECK (category IN ('backlog','unstarted','started','completed','canceled')),
  position INTEGER NOT NULL, color TEXT NOT NULL);
CREATE TABLE projects (id INTEGER PRIMARY KEY, slug TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
  description TEXT, doc_link TEXT, -- both optional; doc_link is usually an obsidian:// link
  start_date TEXT, target_date TEXT, archived_at TEXT, created_at TEXT NOT NULL);
CREATE TABLE milestones (id INTEGER PRIMARY KEY, project_id INTEGER NOT NULL REFERENCES projects ON DELETE CASCADE,
  name TEXT NOT NULL, target_date TEXT);
CREATE TABLE cycles (id INTEGER PRIMARY KEY, number INTEGER NOT NULL UNIQUE, name TEXT,
  start_date TEXT NOT NULL, end_date TEXT NOT NULL);
CREATE TABLE labels (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, color TEXT NOT NULL);
CREATE TABLE issues (id INTEGER PRIMARY KEY, number INTEGER NOT NULL UNIQUE, title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '', status_id INTEGER NOT NULL REFERENCES statuses,
  priority INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 4),
  project_id INTEGER REFERENCES projects, milestone_id INTEGER REFERENCES milestones,
  parent_id INTEGER REFERENCES issues, cycle_id INTEGER REFERENCES cycles,
  start_date TEXT, due_date TEXT, due_time_utc TEXT, due_tz TEXT, estimate_minutes INTEGER,
  properties TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, completed_at TEXT, archived_at TEXT);
CREATE TABLE issue_labels (issue_id INTEGER REFERENCES issues ON DELETE CASCADE,
  label_id INTEGER REFERENCES labels ON DELETE CASCADE, PRIMARY KEY (issue_id, label_id));
CREATE TABLE relations (id INTEGER PRIMARY KEY, from_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  to_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('blocks','related','duplicate')), UNIQUE (from_id, to_id, kind));
CREATE TABLE comments (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  body TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE property_defs (key TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL,
  options TEXT NOT NULL DEFAULT '[]', project_id INTEGER REFERENCES projects, position INTEGER NOT NULL);
CREATE TABLE views (id INTEGER PRIMARY KEY, name TEXT NOT NULL, definition TEXT NOT NULL, position INTEGER NOT NULL);
CREATE TABLE pins (id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('issue','project','view','cycle','note','url')),
  target TEXT NOT NULL, title TEXT, position INTEGER NOT NULL, UNIQUE (kind, target));
CREATE TABLE work_blocks (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  start_utc TEXT NOT NULL, end_utc TEXT NOT NULL);
CREATE TABLE changes (id INTEGER PRIMARY KEY, at TEXT NOT NULL, source TEXT NOT NULL,
  entity TEXT NOT NULL, entity_id TEXT NOT NULL, field TEXT NOT NULL, old_value TEXT, new_value TEXT);
CREATE TABLE import_log (external_id TEXT PRIMARY KEY, source TEXT NOT NULL, issue_id INTEGER NOT NULL,
  imported_at TEXT NOT NULL, source_deleted INTEGER NOT NULL DEFAULT 0);
CREATE TABLE gcal_events (google_event_id TEXT PRIMARY KEY, calendar_id TEXT NOT NULL,
  managed_key TEXT NOT NULL UNIQUE, etag TEXT, last_synced_hash TEXT NOT NULL);
CREATE VIRTUAL TABLE issue_fts USING fts5(title, description, comments, content='', contentless_delete=1, tokenize='porter');

CREATE INDEX idx_milestones_project ON milestones(project_id);
CREATE INDEX idx_issues_status ON issues(status_id);
CREATE INDEX idx_issues_due ON issues(due_date);
CREATE INDEX idx_issues_project ON issues(project_id);
CREATE INDEX idx_issues_milestone ON issues(milestone_id);
CREATE INDEX idx_issues_parent ON issues(parent_id);
CREATE INDEX idx_issues_cycle ON issues(cycle_id);
CREATE INDEX idx_issue_labels_label ON issue_labels(label_id);
CREATE INDEX idx_relations_from ON relations(from_id);
CREATE INDEX idx_relations_to ON relations(to_id);
CREATE INDEX idx_comments_issue ON comments(issue_id);
CREATE INDEX idx_property_defs_project ON property_defs(project_id);
CREATE INDEX idx_work_blocks_issue ON work_blocks(issue_id);
CREATE INDEX idx_changes_entity ON changes(entity, entity_id);
CREATE INDEX idx_import_log_issue ON import_log(issue_id);

INSERT INTO statuses (name, category, position, color) VALUES
  ('Backlog', 'backlog', 0, '#95a2b3'), ('Todo', 'unstarted', 1, '#e2e2e2'),
  ('In Progress', 'started', 2, '#f2c94c'), ('Done', 'completed', 3, '#5e6ad2'),
  ('Canceled', 'canceled', 4, '#95a2b3');
INSERT INTO settings (key, value) VALUES
  ('issue_prefix', '"ZF"'), ('gcal_enabled', 'false'), ('export_enabled', 'false');
INSERT INTO views (name, definition, position) VALUES
  ('Active', '{"filter":{"Cond":{"field":{"Fixed":"StatusCategory"},"op":"In","value":["unstarted","started"]}},"sorts":[],"group_by":null,"layout":"List","columns":[],"include_archived":false}', 0),
  ('Board', '{"filter":{"Cond":{"field":{"Fixed":"StatusCategory"},"op":"In","value":["unstarted","started"]}},"sorts":[],"group_by":{"Fixed":"Status"},"layout":"Board","columns":[],"include_archived":false}', 1),
  ('Timeline: all projects', '{"filter":{"Cond":{"field":{"Fixed":"StatusCategory"},"op":"Neq","value":"completed"}},"sorts":[],"group_by":{"Fixed":"Project"},"layout":"Gantt","columns":[],"include_archived":false}', 2);
