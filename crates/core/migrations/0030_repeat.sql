-- Recurring issues: the repeat rule as text (see crates/core/src/recur.rs); NULL means the issue does not repeat.
ALTER TABLE issues ADD COLUMN repeat_rule TEXT;
