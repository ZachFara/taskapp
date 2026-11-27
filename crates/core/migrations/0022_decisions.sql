-- Decisions: a question with options that someone (the owner) answers. `source` names who raised it ('cli',
-- 'agent:<name>', later 'curia'); `external_id` is unique per source, the hook for a future integration.
-- decision_issues has its own id because row snapshots (rows.rs) need a single primary key.
CREATE TABLE decisions (id INTEGER PRIMARY KEY, question TEXT NOT NULL, context TEXT NOT NULL DEFAULT '',
  options TEXT NOT NULL DEFAULT '[]', recommendation TEXT,
  mode TEXT NOT NULL DEFAULT 'assume' CHECK (mode IN ('blocking', 'assume')),
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'answered', 'withdrawn')),
  answer TEXT, note TEXT NOT NULL DEFAULT '', source TEXT NOT NULL DEFAULT 'cli', external_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, answered_at TEXT);
CREATE UNIQUE INDEX idx_decisions_external ON decisions(source, external_id) WHERE external_id IS NOT NULL;
CREATE INDEX idx_decisions_status ON decisions(status);
CREATE TABLE decision_issues (id INTEGER PRIMARY KEY, decision_id INTEGER NOT NULL REFERENCES decisions ON DELETE CASCADE,
  issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE, UNIQUE (decision_id, issue_id));
CREATE INDEX idx_decision_issues_issue ON decision_issues(issue_id);
