-- Waiting for: who an issue waits on (free text) and the day to check back (NULL when not set).
ALTER TABLE issues ADD COLUMN waiting_on TEXT;
ALTER TABLE issues ADD COLUMN follow_up TEXT;
