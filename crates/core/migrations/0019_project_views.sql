-- A saved view lists issues or projects. Existing views stay issue views.
ALTER TABLE views ADD COLUMN entity TEXT NOT NULL DEFAULT 'issue' CHECK (entity IN ('issue', 'project'));
