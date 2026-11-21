# 0004 Gate design

Context: agents follow what a check enforces and drift from what a document says.

Decision: two parts. Pre-commit hook (`scripts/hooks`, via `core.hooksPath`): layout, boundaries, fmt, clippy, build;
no tests; Rust steps skip when no Rust or manifest file is staged. The commit-msg hook checks the conventional-commit
subject (72 chars). Git runs pre-commit before commit-msg, so the message check cannot literally run first; it is cheap
and still blocks the commit. CI (`.github/workflows/ci.yml`) runs everything on PRs into main and `plan-*`: commits,
script self-tests plus shellcheck, lint (layout, boundaries, fmt, clippy, cargo-machete, build), tests sharded
(core alone, the rest together; split again past 3 minutes, balanced by measured time), UI and Swift jobs when those
files exist, a report-only size step (files over 800 lines, packages over 5000, jscpd duplicates). One aggregate job
`check` is the required status; skipped jobs count as passing, so it reports on every PR. Docs-only changes (every
file under docs/ or .briefs/, or a root .md/.txt, none under `fixtures/`) skip the test jobs. The layout check rejects any
file outside `scripts/layout-allowed.txt`. Dead dependencies are caught by cargo-machete; cargo-deny was not adopted
(it checks licenses and advisories, not unused code). Until T0 the machete `ignored` lists in each crate manifest cover
the not-yet-used dependencies; T0 removes them.

Consequences: `scripts/land.sh` merges only when `check` exists and every check passed. Check scripts have self-tests in
`scripts/test/`.
