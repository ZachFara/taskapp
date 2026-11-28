-- A project's flagged next action. It counts only while that issue is still open and in the project, so
-- finishing or moving the issue needs no write here (and undoing either restores the flag).
ALTER TABLE projects ADD COLUMN next_issue_id INTEGER REFERENCES issues ON DELETE SET NULL;
