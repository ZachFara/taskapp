-- Points are optional and fractional (one decimal, checked in code). Whether they are shown is the `estimation` setting.
ALTER TABLE issues ADD COLUMN points REAL CHECK (points IS NULL OR points >= 0);
INSERT OR IGNORE INTO settings (key, value) VALUES ('estimation', '"both"'), ('point_scale', '"fibonacci"');
