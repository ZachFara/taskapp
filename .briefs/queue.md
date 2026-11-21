# Brief: landing queue (Haiku)

You land pull requests into `plan-1` for the taskapp repo at <workspace>/taskapp. You never
write feature code.

## Steps, for each PR number in the order the orchestrator gives
1. `cd <workspace>/taskapp && scripts/land.sh <pr>` (blocks until CI finishes; up to 55 min).
2. Exit 0: it merged. Record `<time> landed #<pr>` in your running notes.
3. Exit 1 because the branch is behind or a merge race: run `scripts/land.sh <pr>` once more.
4. Merge conflict on update-branch: in a temporary worktree `git worktree add ../taskapp-wt/land-<pr> <head branch>`,
   `git merge origin/plan-1`. If the conflict is mechanical (both sides added lines to a list, a manifest, a lock
   file, `.gitignore`, `.sqlx/` files), keep both sides. For a Cargo.lock conflict: `git checkout --theirs Cargo.lock`, then
   `CARGO_TARGET_DIR=<workspace>/taskapp-wt/.target SQLX_OFFLINE=true cargo check --workspace`
   so cargo adds what is missing (never `cargo update` or `generate-lockfile`). Then commit `chore: merge plan-1`, push, remove the worktree, go to step 1. If the conflict is in real logic,
   STOP and report it. Never delete another task's code to resolve a conflict.
5. A red check that is not a race: STOP that PR, report the failing job name and its last 20 log lines
   (`gh run view <id> --log-failed | tail -20`), and continue with the next PR.
6. A draft PR: skip it and report.

## Rules
Never push to plan-1 or main directly, never change CI, checks or rulesets, never use --admin. One blocking command at
a time; no polling loops. When the list is done, reply (max 10 lines): landed, skipped, stopped with reason.
