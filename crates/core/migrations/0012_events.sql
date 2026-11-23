-- Events are dated things in the owner's local wall-clock time (no zone); an all-day event keeps midnight.
-- event_issues has its own id because row snapshots (rows.rs) need a single primary key.
CREATE TABLE events (id INTEGER PRIMARY KEY, title TEXT NOT NULL, starts_at TEXT NOT NULL, ends_at TEXT,
  all_day INTEGER NOT NULL DEFAULT 0, place TEXT, notes TEXT NOT NULL DEFAULT '', icon TEXT, color TEXT,
  archived INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE event_issues (id INTEGER PRIMARY KEY, event_id INTEGER NOT NULL REFERENCES events ON DELETE CASCADE,
  issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE, UNIQUE (event_id, issue_id));
CREATE INDEX idx_event_issues_issue ON event_issues(issue_id);
CREATE INDEX idx_events_starts ON events(starts_at);
