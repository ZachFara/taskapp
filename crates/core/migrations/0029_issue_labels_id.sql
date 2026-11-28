-- issue_labels gets its own id so row snapshots (rows.rs) can restore a deleted issue's labels on undo.
CREATE TABLE issue_labels_new (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  label_id INTEGER NOT NULL REFERENCES labels ON DELETE CASCADE, UNIQUE (issue_id, label_id));
INSERT INTO issue_labels_new (issue_id, label_id) SELECT issue_id, label_id FROM issue_labels;
DROP TABLE issue_labels;
ALTER TABLE issue_labels_new RENAME TO issue_labels;
CREATE INDEX idx_issue_labels_label ON issue_labels(label_id);
