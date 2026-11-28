-- Cycle retrospective: the goal set at planning, whether it was met (NULL until decided), a 1-5 rating, notes.
ALTER TABLE cycles ADD COLUMN goal TEXT;
ALTER TABLE cycles ADD COLUMN goal_met INTEGER CHECK (goal_met IN (0, 1));
ALTER TABLE cycles ADD COLUMN rating INTEGER CHECK (rating BETWEEN 1 AND 5);
ALTER TABLE cycles ADD COLUMN notes TEXT NOT NULL DEFAULT '';
