# Common rules for the integration tasks (I2a, I2b, I2c)

Read AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, .briefs/P-common.md, then this file and your brief. Read docs/design-system.md and the header comment of ui/src/lib/commands/index.ts on your branch.
- All of plan 2's screens are on `plan-2` now. Worktree: `git -C <workspace>/taskapp fetch && git -C <workspace>/taskapp worktree add ../taskapp-wt/<ID> -b task/<ID> origin/plan-2`.
- Three integration tasks run in parallel. Edit ONLY the files your brief lists. Need a change elsewhere: report the exact lines.
- Load rules: one test file at a time, foreground, `pnpm -C ui exec vitest run <file> --maxWorkers=2`; never the whole suite locally; never kill processes you did not start; do NOT merge plan-2 into your branch (a merge commit triggers a full Rust gate); no harness captures (the orchestrator checks every screen in Chrome); no cargo unless your brief allows it.
- Tests first, each watched failing; record the message. Never weaken a test, the mock or a contract case to get green.
- Tokens only, primitives only, commands only (no keydown listeners in components; a text field's own Enter/Esc handling is fine).
- One PR into plan-2, wait for CI once (`gh pr checks <n> --watch`), rerun once only if the sole failure is a timing test. Do not merge.
- Report: contract format, max 15 lines, start/finish times, PR number, and for each item: done / not done and why.

## Visual review findings (orchestrator looked at every screen; captures in <workspace>/taskapp-wt/batch3/ui/harness-out/*.png, open the ones for your screens with the Read tool BEFORE you start)
The app is coherent, but these are wrong. Your brief lists which are yours. A fix is done when the screen looks right, so
you MAY take ONE mock-mode capture per fixed screen at the end to check it by eye:
`pnpm -C ui shot <route> --mode mock --theme light --size 1440x900` (free ports; no cargo; mock only), open it with Read, fix, done.
