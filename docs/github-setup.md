# GitHub setup (run by the owner or orchestrator, in order)

The repo is approved as PUBLIC. Nothing here has been run. Replace OWNER with the GitHub login.

```sh
# 1. Create the repo and push main and the setup branch
gh repo create OWNER/taskapp --public --source . --remote origin
git push origin main chore/setup

# 2. Protect main: PRs only, `check` required, branch up to date, no force pushes, no deletion
gh api -X PUT repos/OWNER/taskapp/branches/main/protection --input - <<'JSON'
{
  "required_status_checks": { "strict": true, "contexts": ["check"] },
  "enforce_admins": true,
  "required_pull_request_reviews": { "required_approving_review_count": 0 },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
JSON

# 3. Ruleset: `check` required on integration branches (plan-*)
gh api -X POST repos/OWNER/taskapp/rulesets --input - <<'JSON'
{
  "name": "integration-branches",
  "target": "branch",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["refs/heads/plan-*"], "exclude": [] } },
  "rules": [
    { "type": "pull_request", "parameters": { "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": false, "require_code_owner_review": false,
        "require_last_push_approval": false, "required_review_thread_resolution": false } },
    { "type": "required_status_checks", "parameters": { "strict_required_status_checks_policy": true,
        "required_status_checks": [ { "context": "check" } ] } },
    { "type": "non_fast_forward" }
  ]
}
JSON

# 4. Open the setup PR
gh pr create --base main --head chore/setup --title "chore: project setup" --fill
```

Notes: the `check` context only appears after CI has run once, so run step 4 before step 2 if GitHub refuses an
unknown context. Public repo: nothing secret may ever be committed; `.env` holds only the SQLx DATABASE_URL.
`pull_request` is required in the ruleset so integration branches cannot be pushed to directly; drop that rule if
task PRs must be pushed straight into plan-N by automation (then `check` only guards the final PR).
