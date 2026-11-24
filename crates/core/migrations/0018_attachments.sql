-- Pasted images. The bytes live in the attachments folder next to the database, named `file` (content hash plus
-- extension); a markdown body points at one with `attachment:<id>`. The same bytes are stored once.
CREATE TABLE attachments (
    id INTEGER PRIMARY KEY,
    sha256 TEXT NOT NULL UNIQUE,
    file TEXT NOT NULL,
    mime TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
