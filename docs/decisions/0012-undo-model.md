# 0012 Undo and redo: steps over the change history

Decision: every command call that writes is one step, whatever it touches. `changes` rows carry a step id and label.
Field edits store old and new values; creates, deletes and link rows are stored as whole-row snapshots (`row:<table>`),
so undo restores the same ids and links. Undo applies a step in reverse in one `BEGIN IMMEDIATE` transaction, all or
nothing. Redo reapplies it; a new change ends the redo stack. Undo and redo are recorded with their own source and are
never steps themselves.

Conflicts: if a value no longer equals what the step left behind, undo refuses the whole step and names what changed.
It never overwrites.

Scope: issues, labels, comments, projects, milestones, cycles, statuses (including delete-with-move and reorder),
property definitions, relations, views and their display settings, view templates, pins, issue templates, icons and
colours, work blocks, settings, points, bulk edits, and a whole import. Issues are never hard-deleted, so undoing a
create archives. Sync side effects (calendar, reminders, export) are not undone; the next sync reconciles.

Why not a command log with inverse commands: every writer would need a hand-written inverse, and agents (MCP, CLI)
write through the same core, so recording at the row level covers them without extra work.

Known limits: statuses created by an import stay after undo; the mock mirrors the model and is held to it by contract
tests.
