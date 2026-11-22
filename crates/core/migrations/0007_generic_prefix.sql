-- The old default prefix (set by 0001) becomes ABC only on a database that has no issues yet.
-- Existing databases with issues keep their key: the prefix is display-only, but it is theirs.
UPDATE settings SET value = '"ABC"'
WHERE key = 'issue_prefix' AND value = '"ZF"' AND NOT EXISTS (SELECT 1 FROM issues);
