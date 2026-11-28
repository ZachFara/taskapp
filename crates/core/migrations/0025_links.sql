-- Links on an issue: a URL (or a taskapp:// link) with a title and a kind, e.g. the pull request that solved it.
-- One row per (issue, url) so recording the same solution twice is a no-op.
CREATE TABLE links (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  url TEXT NOT NULL, title TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL DEFAULT 'other' CHECK (kind IN ('solution', 'reference', 'pr', 'commit', 'doc', 'other')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE (issue_id, url));
