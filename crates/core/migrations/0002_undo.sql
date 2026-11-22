-- A step is everything one command call changed. undone: 0 live, 1 undone (can be redone), 2 undone and
-- no longer redoable because something changed afterwards. Rows with no step (undo, redo, imports) are not undoable.
ALTER TABLE changes ADD COLUMN step INTEGER;
ALTER TABLE changes ADD COLUMN step_label TEXT;
ALTER TABLE changes ADD COLUMN undone INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_changes_step ON changes(step);
