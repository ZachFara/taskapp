# Linear migration runbook

Give the prompt below to an agent that has two MCP servers connected: the official Linear server (read only) and
`taskapp-mcp` (write). The agent never changes anything in Linear. Take a snapshot of the taskapp database first
(`taskapp backup` or copy the file with the app quit); a failed run is undone with `taskapp backup restore <file> --yes`.

Tool names below are the ones in `crates/mcp/src/tools.rs`. Linear tool names are the official server's.

## Prompt

> You are migrating my Linear workspace into taskapp. Rules:
> 1. Linear is read only. Use only Linear's list/get tools (`list_teams`, `list_issue_statuses`, `list_issue_labels`,
>    `list_projects`, `list_milestones`, `list_cycles`, `list_issues`, `list_comments`, `get_issue`). Never call a
>    Linear `save_*`, `delete_*` or comment tool. Page through every list until `hasNextPage` is false.
> 2. Write only through `taskapp-mcp`. Keep a map `linear id -> taskapp id` for every object you create.
> 3. Create in this order, one kind at a time, and stop and report on the first error:
>    a. `create_status` (name, category, color) for each Linear status. Map Linear types to the categories
>       Backlog, Unstarted, Started, Completed, Canceled. Check `list_statuses` first and reuse a status of the same name.
>    b. `create_label` (name, color) for each label; check `list_labels` first.
>    c. `create_project` (slug, name) for each project, then `update_project` (id, description, start_date,
>       target_date, doc_link) for the rest of its fields; then `create_milestone` (project_id, name, target_date).
>    d. `create_cycle` (start_date, end_date, name).
>    e. `create_property` for every Linear custom field you want to keep, and always
>       `create_property` with key `linear_id`, name `Linear ID`, kind `Text`. Check `list_properties` first.
> 4. Then issues, oldest first. For each, `create_issue` with `migration: true`, title, description, status_id,
>    priority, project_id, milestone_id, cycle_id, label_ids, start_date, due, estimate_minutes, `created_at` and
>    `completed_at` copied from Linear, and `properties` including `linear_id` (the Linear identifier, e.g. ENG-12).
>    Skip an issue whose `linear_id` already exists (`search_issues`, then `get_issue`), so the run can be repeated.
> 5. Then links, each with `migration: true` where the tool accepts it: sub-issues with `update_issue`
>    (key, patch `{parent_id}`); relations with `add_relation` (from_key, to_key, kind Blocks, Related or Duplicate);
>    comments with `add_comment` (key, body, `created_at` from Linear).
> 6. Finish with a count comparison per project: issues, open issues, comments and relations in Linear versus
>    taskapp (`query_view` or `search_issues` for taskapp). Print a table. A mismatch is a failure: list the missing
>    `linear_id`s and do not retry blindly.

## After the run

1. Open the app; spot check five issues across projects, one with a sub-issue, one with a relation, one with comments.
2. History of migrated rows shows source `Migration` (`get_history` on any issue).
3. Keep `linear_id` for traceability; delete the property only when you no longer need it (`delete_property`
   snapshots first).
