CREATE TABLE issue_templates (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, slug TEXT NOT NULL UNIQUE,
  icon TEXT, color TEXT, body TEXT NOT NULL); -- body: JSON, see templates::TemplateBody
