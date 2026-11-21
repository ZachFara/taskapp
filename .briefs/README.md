# Briefs

One file per task: `.briefs/T<n>.md` (setup brief: `setup.md`). A stopped agent restarts from one line:
"Read .briefs/T<n>.md and do exactly what it says." A brief points at files and does not restate them. It names:
files the task owns, files to stay out of, the plan section to read, the hostile cases the tests must cover, the
model and why, and the 15-line reply cap. Briefs are tracked (not ignored) so they survive restarts.
