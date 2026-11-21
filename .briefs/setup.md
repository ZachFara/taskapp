# Brief: Phase 1 setup (one agent, no feature code)

Read first: docs/ORCHESTRATOR-PROMPT.txt (sections PHASE 1, ENGINEERING, CODE, MY MACHINE, DECISIONS, PARALLEL EXECUTION), docs/DESIGN.md, docs/plans/plan-1.md (the "Global Constraints", "How to run this in parallel", "File structure and ownership" sections and Task T0).

## Settings (fill-ins for the prompt's OWNER SETTINGS)
- Project: taskapp. Spec: docs/DESIGN.md. Plan: docs/plans/plan-1.md. Decisions: docs/decisions/.
- Toolchain: Rust stable (cargo 1.99 present), clippy + rustfmt, Swift 6.3 present, node 26 present. pnpm, sqlx-cli and cargo-tauri are NOT installed: do NOT install anything globally. Use `npx -y pnpm@10` for pnpm. Write scripts/install-tools.sh (prints what it will do; the user runs it) for pnpm, sqlx-cli, tauri-cli. CI installs its own.
- Models: orchestrator = the session model. All subagents Sonnet, never Opus (user rule). Planned split mid-tier/smallest both map to Sonnet; record the intended split in PROGRESS.md Model column anyway.
- Merging: no GitHub remote exists yet and creating a repo is the user's call. Work on branch chore/setup in this local repo, commit there, and do NOT create or push to any remote. Write .github/workflows/ci.yml (aggregate job `check`), the PR template, and scripts/land.sh as specified, and a docs/github-setup.md listing exact `gh` commands for repo creation, branch protection, and the integration-branch ruleset for the user to run.
- Questions: Curia decision tickets (not available to you; list open questions in PROGRESS.md instead).
- Third-party tools driven: Apple Reminders (EventKit), Google Calendar API, Linear MCP (read-only, migration only). No probes now; list them under "Probes needed" in PROGRESS.md (T10 reminders helper, T24 digest reminder encoding).
- My machine: never touch ~/Library/Application Support/taskapp, the user's Reminders, Google account, Keychain, or Linear. Everything under ~/Ark/Development/active/taskapp only.

## Deliver
1. AGENTS.md (~150 lines, map not diary) and CLAUDE.md = "See AGENTS.md".
2. PROGRESS.md: status; every task T0..T29 with done criteria and Model column; Open Questions; Rulings; After this plan; next steps.
3. docs/decisions/0001+ short records: toolchain, import-boundary rules, registration points, gate design (pre-commit vs CI), no-global-install rule.
4. Checks from day one, matched to this plan's layout: clippy pedantic-ish with complexity/function-length limits and no `allow` suppressions, cargo-deny or equivalent for unused deps, cargo-machete for dead deps, import-boundary check (core must not depend on adapters/sync crates; mcp, cli, commands depend only on core; sync crates depend on core), a layout check against an allowed-files list, conventional-commit check, report-only size step (800 lines/file, 5000/package, duplicates).
5. Pre-commit hook (git hooks path in repo, scripts/): commit-msg check first, then fmt, clippy, layout, build; NO tests; under a minute.
6. CI: all checks per PR into main and into plan-* branches, one aggregate job `check`, sharding rule, docs-only skip.
7. scripts/land.sh as the prompt specifies. Test it with a shellcheck pass and a dry-run mode.
8. Cargo workspace skeleton ONLY as far as plan T0 Step 1 says (manifests with ALL dependencies pinned, empty crates compile). No schema, no types, no stubs: that is task T0.
9. .gitignore (.scratch/, .briefs not ignored, target/, node_modules, harness-out/, agent dirs), implementer contract at docs/IMPLEMENTER-CONTRACT.md, brief-file convention in .briefs/.
10. Registration points: per plan, T0 owns module declarations; note in AGENTS.md.
11. Commit on chore/setup, conventional commits, subjects <=72 chars, end each with:
   Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
12. Verify: run the pre-commit hook script and `cargo build --workspace`. Do not run the full test suite.

## Hostile cases to cover
land.sh refuses a non-green PR and a draft PR; layout check rejects a stray file; import-boundary check fails when core imports a sync crate (add a tiny test of each check, not just the happy path).

## Stay out of
~ outside the project dir, any remote, GitHub, global installs.

## Reply (max 15 lines)
What was done, what was verified, what was skipped or not checked, any risk. List every default/number you invented with the rejected alternative.
