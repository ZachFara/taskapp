# 0013 The UI mock must equal the backend, proven by contract tests

Context: plan 1's screen checks passed against a mock that ignored filters, and every saved view was empty on the real
database.

Decision: one table of cases (command, arguments, expected result) runs against the mock and against a real
`taskapp-devserver` on a freshly seeded database; any difference fails. The mock and the seed hold the same fixture
workspace. CI builds the devserver before the UI tests so the real side always runs.

What it caught while plan 2 was built: status categories stored lowercase while a filter sent "Completed"; a raw
UNIQUE error where the mock returned a tidy conflict; a mock redo that never restored a created issue; cycle progress
that counted work done before the cycle.

Limit: a case exists only where someone wrote one. The final review listed UI-reachable commands with no case; the fix
wave added the highest-risk ones, and the rest are listed in PROGRESS.md.
