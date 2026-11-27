-- Status management: a short definition, a flow kind for flow efficiency (active work, waiting, or idle) and an
-- optional stale-nudge age in days. Started statuses are active; everything else starts idle.
ALTER TABLE statuses ADD COLUMN definition TEXT NOT NULL DEFAULT '';
ALTER TABLE statuses ADD COLUMN flow_kind TEXT NOT NULL DEFAULT 'idle' CHECK (flow_kind IN ('active','waiting','idle'));
ALTER TABLE statuses ADD COLUMN nudge_days INTEGER CHECK (nudge_days IS NULL OR nudge_days > 0);
UPDATE statuses SET flow_kind = 'active' WHERE category = 'started';
