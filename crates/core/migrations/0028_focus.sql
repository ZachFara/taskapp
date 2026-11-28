-- Focus: an issue's depth (NULL until chosen) and one row per focus session. A session is open while ended_at is
-- NULL; the unique index on the expression lets only one row be open at a time.
ALTER TABLE issues ADD COLUMN depth TEXT CHECK (depth IN ('deep', 'shallow'));
CREATE TABLE focus_sessions (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  started_at TEXT NOT NULL, ended_at TEXT, deep INTEGER NOT NULL DEFAULT 0 CHECK (deep IN (0, 1)));
CREATE INDEX idx_focus_sessions_issue ON focus_sessions(issue_id);
CREATE UNIQUE INDEX idx_focus_sessions_open ON focus_sessions((ended_at IS NULL)) WHERE ended_at IS NULL;
