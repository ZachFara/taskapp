# Final review of plan 2 (one of four reviewers, split by area)
Read AGENTS.md, docs/plans/plan-2.md (goals and "Review focus"), docs/DESIGN.md, docs/design-system.md. You review the code AS IT STANDS on `origin/plan-2` in your own worktree:
`git -C <workspace>/taskapp fetch && git -C <workspace>/taskapp worktree add ../taskapp-wt/<ID> --detach origin/plan-2`
Rules:
- You change NO product code and open no PR. You may write throwaway tests/scripts in your worktree to reproduce a finding; do not commit them.
- Reproduce every finding before you report it (a failing test, a command and its output, or a headless-browser step with what you saw). No finding from reading alone, except security findings with a concrete input.
- Break three safety-relevant behaviours per feature in your area ON PURPOSE (edit the code locally, run the tests that should notice) and say whether a test noticed. Revert each.
- Write your report AS YOU GO to <workspace>/taskapp/.scratch/review2/<ID>.md (create the folder). Format per finding: id, severity (Critical = data loss, security, or a core flow broken; Important = wrong behaviour a user will hit or a missing test for a safety behaviour; Minor = polish), where (file:line), what happens, how you reproduced it (exact steps), suggested fix in one line. End with: what you did not look at.
- Load rules: one test target or file at a time (`cargo test -p <crate> --test <name> -j 4`, `pnpm -C ui exec vitest run <file> --maxWorkers=2`); never kill processes you did not start; no full suites.
- Never touch `~/Library/Application Support/taskapp`, real Reminders, Google, Keychain, the real Linear API, or any network service. Use temp dirs and the devserver (`cargo run -p taskapp-devserver -- seed --db <tmp>` then `serve --db <tmp> --port <free> --allow-origin <origin>`).
- Time box: about 40 minutes. Report what you have. Final message: counts by severity and the path of your report (max 10 lines).
Known and already on the fix list (do not re-report, but say if you see them getting worse): step labels in the undo toast lack counts and one reads "Issues create from quickadd"; the toast persists across navigation; App.test `esc_closes_an_open_editor_first...` is flaky in CI; changing the time zone in setup needs an app restart; git export had no points (being fixed); Linear import never ran live.
