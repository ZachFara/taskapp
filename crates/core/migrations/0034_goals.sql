-- Goals above projects (GTD horizons of focus): a few per quarter, linked many-to-many to projects.
-- goal_projects has its own id because row snapshots (rows.rs) need a single primary key.
CREATE TABLE goals (id INTEGER PRIMARY KEY, title TEXT NOT NULL, quarter TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'achieved', 'dropped')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE goal_projects (id INTEGER PRIMARY KEY, goal_id INTEGER NOT NULL REFERENCES goals ON DELETE CASCADE,
  project_id INTEGER NOT NULL REFERENCES projects ON DELETE CASCADE, UNIQUE (goal_id, project_id));
CREATE INDEX idx_goal_projects_project ON goal_projects(project_id);
