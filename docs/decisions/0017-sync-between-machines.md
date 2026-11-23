# 0017 Sync between machines through the export, restore, and why backups stay

Context: the export folder is a Git repository, so two machines can share it. The later list asked for two-way sync, a
restore command, a complete export and a decision on backups.

Decision:
- The export now also carries work blocks (in each issue's frontmatter) and the import log's deleted flag. Left out on
  purpose: secrets, machine-local settings and paths, the Google event map, and the raw change history.
- Restore is `taskapp restore --from <dir> --db <new path>`, the existing rebuild; it refuses an existing database.
- Export fetches first. Remote ahead: refuse to write, show the message, offer "Pull and apply". Diverged: refuse, say
  so, a person merges with git. Pull and apply fast-forwards, then applies the incoming issue files as one undoable
  step "Sync from remote", last writer wins per issue by `updated_at` (equal or older incoming loses and is reported).
  A key that names a different issue on the two machines (different `created_at`) is reported and kept local. The issue
  keeps the file's updated time, so the next export is identical and nothing bounces back.
- Backups stay until the owner has proven restore on real data. Backups are the whole database including history;
  the export is not.

Left for later (the safe subset above is what exists):
- Applying statuses, labels, projects, cycles, properties, views, templates, pins and settings from the remote. Until
  then the next export from a machine writes its own version of those files over the remote's.
- Deletions: nothing is removed on pull (comments, relations, work blocks, issues are only added or changed).
- Two machines creating a new issue at the same time get the same key; the second one to pull reports a conflict.
- A three-way merge (this needs a base copy) so that edits to different fields of one issue both survive.
