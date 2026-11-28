//! Applies the issue files of a pulled export to the database, as one undoable step.
//!
//! Last writer wins per issue by `updated_at`. Comments, relations and work blocks are only ever added.
//! Settings, statuses, labels, projects and the other list files are not applied: see
//! docs/decisions/0017-sync-between-machines.md.

use std::{collections::HashMap, path::Path};

use chrono::{DateTime, NaiveDateTime, Utc};
use taskapp_core::{
    Applied, ChangeSource, Db, Depth, Due, Error, EventInput, IssueId, IssuePatch, Result, blocks,
    comments, events, history, issues, relation_kinds, relations,
};

use crate::{
    files::{
        CommentText, EventFile, IssueFront, RelationKindRow, invalid, parse_issue, plain_dir,
        read_plain,
    },
    git::{self, RemoteState},
    rebuild::{files_in, read_list, read_yaml},
};

const SRC: ChangeSource = ChangeSource::System;

type Names = HashMap<String, i64>;

/// What Settings shows when another machine pushed first. The UI offers "Pull and apply".
pub const REMOTE_AHEAD: &str = "The remote has changes from another machine. Choose Pull and apply in Settings > Export; nothing was overwritten.";

/// Writes the export, commits it and pushes it. With a remote, first looks for commits this machine has not
/// seen and refuses to write over them.
///
/// # Errors
///
/// `Invalid` with [`REMOTE_AHEAD`] when the remote is ahead, or when the histories diverged; otherwise as
/// [`crate::write::write_snapshot`] and [`git::commit_and_push`].
pub async fn export(db: &Db, dir: &Path, remote: Option<&str>) -> Result<()> {
    if let Some(remote) = remote {
        match git::remote_state(dir, remote).await? {
            RemoteState::Current => {}
            RemoteState::Behind => return Err(Error::Invalid(REMOTE_AHEAD.into())),
            RemoteState::Diverged => return Err(Error::Invalid(DIVERGED.into())),
        }
    }
    crate::write::write_snapshot(db, dir).await?;
    git::commit_and_push(dir, "export", remote).await?;
    Ok(())
}

const DIVERGED: &str =
    "This machine and the remote both have new commits; merge them in the export folder first.";

/// Fast-forwards the export folder to the remote and applies the incoming issues.
/// `created` of the result counts issues created or changed; `warnings` are the conflicts.
///
/// # Errors
///
/// `Invalid` when the histories diverged; git and database errors otherwise. A single issue that cannot be
/// applied is reported as a conflict instead.
pub async fn pull_and_apply(db: &Db, dir: &Path, remote: &str) -> Result<Applied> {
    match git::remote_state(dir, remote).await? {
        RemoteState::Current => return Ok(empty()),
        RemoteState::Diverged => {
            return Err(Error::Invalid(DIVERGED.into()));
        }
        RemoteState::Behind => {
            git::reject_links(dir).await?;
            git::fast_forward(dir).await?;
        }
    }
    history::step("Sync from remote", apply_dir(db, dir)).await
}

async fn names(db: &Db, sql: &str) -> Result<Names> {
    Ok(sqlx::query_as::<_, (String, i64)>(sql)
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .collect())
}

/// Name lookups for one pass; a name the database lacks makes the issue a conflict.
struct Lookup {
    statuses: Names,
    projects: Names,
    milestones: Names,
    cycles: Names,
    labels: Names,
    issues: Names,
}

impl Lookup {
    async fn load(db: &Db) -> Result<Lookup> {
        Ok(Lookup {
            statuses: names(db, "SELECT name, id FROM statuses").await?,
            projects: names(db, "SELECT slug, id FROM projects").await?,
            milestones: names(
                db,
                "SELECT p.slug || '/' || m.name, min(m.id) FROM milestones m
                 JOIN projects p ON p.id = m.project_id GROUP BY 1",
            )
            .await?,
            cycles: names(db, "SELECT CAST(number AS TEXT), id FROM cycles").await?,
            labels: names(db, "SELECT name, id FROM labels").await?,
            issues: names(db, "SELECT CAST(number AS TEXT), id FROM issues").await?,
        })
    }
}

fn number_of(key: &str) -> Result<i64> {
    key.rsplit_once('-')
        .and_then(|(_, n)| n.parse().ok())
        .ok_or_else(|| invalid(format!("{key:?} is not an issue key")))
}

fn find(map: &Names, what: &str, name: &str) -> Result<i64> {
    map.get(name)
        .copied()
        .ok_or_else(|| Error::Invalid(format!("unknown {what} {name:?}")))
}

fn due_of(f: &IssueFront) -> Option<Due> {
    match (f.due_time, &f.due_tz, f.due_date) {
        (Some(utc), Some(tz), _) => Some(Due::DateTime {
            utc,
            tz: tz.clone(),
        }),
        (_, _, Some(date)) => Some(Due::Date(date)),
        _ => None,
    }
}

/// The patch that turns `old` into the file's issue; empty when they agree.
fn patch_of(
    f: &IssueFront,
    desc: &str,
    old: &taskapp_core::Issue,
    l: &Lookup,
) -> Result<IssuePatch> {
    let mut p = IssuePatch::default();
    let status = taskapp_core::StatusId(find(&l.statuses, "status", &f.status)?);
    let priority = u8::try_from(f.priority).map_err(invalid)?;
    if f.title != old.title {
        p.title = Some(f.title.clone());
    }
    if desc != old.description {
        p.description = Some(desc.to_string());
    }
    if status != old.status_id {
        p.status_id = Some(status);
    }
    if priority != old.priority {
        p.priority = Some(priority);
    }
    if f.start_date != old.start_date {
        p.start_date = Some(f.start_date);
    }
    if due_of(f) != old.due {
        p.due = Some(due_of(f));
    }
    let estimate = f
        .estimate_minutes
        .map(u32::try_from)
        .transpose()
        .map_err(invalid)?;
    if estimate != old.estimate_minutes {
        p.estimate_minutes = Some(estimate);
    }
    if f.points != old.points {
        p.points = Some(f.points);
    }
    if f.hidden_until != old.hidden_until {
        p.hidden_until = Some(f.hidden_until);
    }
    let depth = match f.depth.as_deref() {
        None => None,
        Some("deep") => Some(Depth::Deep),
        Some("shallow") => Some(Depth::Shallow),
        Some(other) => return Err(invalid(format!("depth {other} is not deep or shallow"))),
    };
    if depth != old.depth {
        p.depth = Some(depth);
    }
    if f.repeat != old.repeat {
        p.repeat = Some(f.repeat.clone());
    }
    if f.waiting_on != old.waiting_on {
        p.waiting_on = Some(f.waiting_on.clone());
    }
    if f.follow_up != old.follow_up {
        p.follow_up = Some(f.follow_up);
    }
    patch_refs(&mut p, f, old, l)?;
    patch_props(&mut p, f, old);
    Ok(p)
}

/// The fields that name other rows: project, milestone, cycle, parent and labels.
fn patch_refs(
    p: &mut IssuePatch,
    f: &IssueFront,
    old: &taskapp_core::Issue,
    l: &Lookup,
) -> Result<()> {
    let project = f
        .project
        .as_ref()
        .map(|n| find(&l.projects, "project", n).map(taskapp_core::ProjectId))
        .transpose()?;
    let milestone = match (&f.project, &f.milestone) {
        (Some(p), Some(m)) => Some(taskapp_core::MilestoneId(find(
            &l.milestones,
            "milestone",
            &format!("{p}/{m}"),
        )?)),
        _ => None,
    };
    let cycle = f
        .cycle
        .map(|n| find(&l.cycles, "cycle", &n.to_string()).map(taskapp_core::CycleId))
        .transpose()?;
    let parent = match &f.parent {
        Some(k) => Some(IssueId(find(
            &l.issues,
            "parent",
            &number_of(k)?.to_string(),
        )?)),
        None => None,
    };
    let mut labels = f
        .labels
        .iter()
        .map(|n| find(&l.labels, "label", n).map(taskapp_core::LabelId))
        .collect::<Result<Vec<_>>>()?;
    labels.sort_by_key(|l| l.0);
    if project != old.project_id {
        p.project_id = Some(project);
    }
    if milestone != old.milestone_id {
        p.milestone_id = Some(milestone);
    }
    if cycle != old.cycle_id {
        p.cycle_id = Some(cycle);
    }
    if parent != old.parent_id {
        p.parent_id = Some(parent);
    }
    if labels != old.label_ids {
        p.label_ids = Some(labels);
    }
    Ok(())
}

/// Custom property values; a key the file lacks is cleared.
fn patch_props(p: &mut IssuePatch, f: &IssueFront, old: &taskapp_core::Issue) {
    let mut props = f.properties.clone();
    for k in old
        .properties
        .keys()
        .filter(|k| !f.properties.contains_key(*k))
    {
        props.insert(k.clone(), serde_json::Value::Null);
    }
    let null = serde_json::Value::Null;
    if props
        .iter()
        .any(|(k, v)| old.properties.get(k).unwrap_or(&null) != v)
    {
        p.properties = Some(props);
    }
}

type Span = (DateTime<Utc>, DateTime<Utc>);

/// An issue file as read: frontmatter, description, comments.
type Incoming = (IssueFront, String, Vec<CommentText>);

fn read_all(dir: &Path) -> Result<Vec<Incoming>> {
    if !plain_dir(dir, "issues")? {
        return Ok(vec![]);
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir.join("issues"))? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == "md") {
            paths.push(entry.path());
        }
    }
    paths.sort();
    paths.iter().map(|p| parse_issue(&read_plain(p)?)).collect()
}

/// Creates the issue under the number the file names, so keys agree on both machines.
async fn create_missing(db: &Db, number: i64, f: &IssueFront, desc: &str) -> Result<()> {
    let created = issues::create(
        db,
        taskapp_core::NewIssue {
            title: f.title.clone(),
            description: Some(desc.to_string()),
            created_at: Some(f.created_at),
            ..Default::default()
        },
        SRC,
    )
    .await?;
    sqlx::query("UPDATE issues SET number = ?, updated_at = ? WHERE id = ?")
        .bind(number)
        .bind(f.created_at)
        .bind(created.id.0)
        .execute(&db.pool)
        .await?;
    Ok(())
}

async fn apply_dir(db: &Db, dir: &Path) -> Result<Applied> {
    let incoming = read_all(dir)?;
    let mut out = empty();
    let mut fresh = std::collections::HashSet::new();
    let mut local = Lookup::load(db).await?.issues;
    for (f, desc, _) in &incoming {
        let number = number_of(&f.key)?;
        if !local.contains_key(&number.to_string()) {
            match create_missing(db, number, f, desc).await {
                Ok(()) => {
                    out.created += 1;
                    fresh.insert(number);
                }
                Err(e) => out.warnings.push(format!("{}: not created: {e}", f.key)),
            }
        }
    }
    let l = Lookup::load(db).await?;
    local = l.issues.clone();
    for (f, desc, comments) in &incoming {
        let number = number_of(&f.key)?;
        let Some(&id) = local.get(&number.to_string()) else {
            continue;
        };
        match apply_issue(
            db,
            &l,
            IssueId(id),
            (f, desc, comments),
            fresh.contains(&number),
        )
        .await
        {
            Ok(changed) => out.created += u32::from(changed && !fresh.contains(&number)),
            Err(e) => out.warnings.push(format!("{}: {e}", f.key)),
        }
    }
    out.warnings.extend(add_extras(db, dir).await);
    for (f, _, _) in &incoming {
        let Some(&id) = local.get(&number_of(&f.key)?.to_string()) else {
            continue;
        };
        if let Err(e) = add_links(db, &l, IssueId(id), f).await {
            out.warnings.push(format!("{}: {e}", f.key));
        }
    }
    out.warnings.extend(
        add_events(db, dir, &l)
            .await
            .err()
            .map(|e| format!("events: {e}")),
    );
    Ok(out)
}

/// Relation kinds and images the files carry; what fails is reported, the rest still applies.
async fn add_extras(db: &Db, dir: &Path) -> Vec<String> {
    let kinds = add_kinds(db, dir)
        .await
        .err()
        .map(|e| format!("relation kinds: {e}"));
    let images = add_images(db, dir)
        .await
        .err()
        .map(|e| format!("images: {e}"));
    kinds.into_iter().chain(images).collect()
}

/// Custom relation kinds the file names and this database lacks; nothing is ever removed.
async fn add_kinds(db: &Db, dir: &Path) -> Result<()> {
    let have = relation_kinds::list(db).await?;
    for k in read_list::<RelationKindRow>(&dir.join("relation_kinds.yaml"))? {
        if !have.iter().any(|h| h.id.0 == k.id) {
            relation_kinds::create(db, &k.name, &k.forward, &k.reverse, k.symmetric, SRC).await?;
        }
    }
    Ok(())
}

/// Images the files name and this database lacks, under the ids the text points at. An id this machine already
/// uses keeps its own image (shortcut: ids are not remapped, so two machines that paste at once can clash).
async fn add_images(db: &Db, dir: &Path) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    crate::rebuild::restore_attachments(db, &mut tx, dir).await?;
    tx.commit().await?;
    Ok(())
}

/// Events the files name and this database lacks (same title and start), with their issue links.
async fn add_events(db: &Db, dir: &Path, l: &Lookup) -> Result<()> {
    let at = |s: &str| {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
            .map_err(invalid)
    };
    let mut have = events::list(db, None, None).await?;
    for path in files_in(dir, "events", "yaml")? {
        let e: EventFile = read_yaml(&path)?;
        let starts_at = at(&e.starts_at)?;
        let known = have
            .iter()
            .find(|h| h.title == e.title && h.starts_at == starts_at)
            .cloned();
        let event = if let Some(h) = known {
            h
        } else {
            let input = EventInput {
                title: e.title,
                starts_at,
                ends_at: e.ends_at.as_deref().map(at).transpose()?,
                all_day: e.all_day,
                place: e.place,
                notes: e.notes,
                icon: e.icon,
                color: e.color,
                archived: e.archived,
            };
            let made = events::create(db, input, SRC).await?;
            have.push(made.clone());
            made
        };
        for key in &e.issues {
            let issue = IssueId(find(&l.issues, "issue", &number_of(key)?.to_string())?);
            events::link(db, event.id, issue, SRC).await?;
        }
    }
    Ok(())
}

/// `Ok(true)` when the issue was created or changed. A differing issue the local copy is newer than stays as it is.
async fn apply_issue(
    db: &Db,
    l: &Lookup,
    id: IssueId,
    (f, desc, comments): (&IssueFront, &str, &[CommentText]),
    fresh: bool,
) -> Result<bool> {
    let old = issues::get(db, id).await?;
    if old.created_at != f.created_at {
        return Err(Error::Invalid(
            "a different issue has this key on the other machine; kept this machine's".into(),
        ));
    }
    let mut patch = patch_of(f, desc, &old, l)?;
    let archived = f.archived_at.is_some() != old.archived_at.is_some();
    let differs = patch != IssuePatch::default() || archived;
    if differs && !fresh && f.updated_at <= old.updated_at {
        return Err(Error::Invalid(
            "changed on both machines; kept this machine's version".into(),
        ));
    }
    if patch == IssuePatch::default() && !archived && comments.is_empty() {
        return Ok(false);
    }
    if differs {
        issues::update(db, id, std::mem::take(&mut patch), SRC).await?;
        if archived && f.archived_at.is_some() {
            issues::archive(db, id, SRC).await?;
        } else if archived {
            issues::restore(db, id, SRC).await?;
        }
        sqlx::query("UPDATE issues SET updated_at = ? WHERE id = ?")
            .bind(f.updated_at)
            .bind(id.0)
            .execute(&db.pool)
            .await?;
    }
    add_comments(db, id, comments).await?;
    Ok(differs)
}

async fn add_comments(db: &Db, id: IssueId, incoming: &[CommentText]) -> Result<()> {
    let have: Vec<(DateTime<Utc>, String)> = comments::list(db, id)
        .await?
        .into_iter()
        .map(|c| (c.created_at, c.body))
        .collect();
    for c in incoming {
        if !have
            .iter()
            .any(|(at, body)| *at == c.created_at && *body == c.body)
        {
            comments::add_comment(db, id, &c.body, Some(c.created_at), SRC).await?;
        }
    }
    Ok(())
}

/// Relations and work blocks named by the file and missing here; nothing is ever removed.
async fn add_links(db: &Db, l: &Lookup, id: IssueId, f: &IssueFront) -> Result<()> {
    use taskapp_core::{Relation, RelationKind};
    let have = relations::for_issue(db, id).await?;
    let builtin = [
        ("blocks", &f.blocks),
        ("related", &f.related),
        ("duplicate", &f.duplicate),
    ];
    let custom = f.relations.iter().map(|(k, v)| (k.as_str(), v));
    for (kind, targets) in builtin.into_iter().chain(custom) {
        let kind = RelationKind::new(kind);
        for t in targets {
            let to = IssueId(find(&l.issues, "issue", &number_of(t)?.to_string())?);
            let rel = Relation {
                from: id,
                to,
                kind: kind.clone(),
            };
            if !have.contains(&rel) {
                relations::add(db, rel, SRC).await?;
            }
        }
    }
    let blocks_have: Vec<Span> =
        sqlx::query_as("SELECT start_utc, end_utc FROM work_blocks WHERE issue_id = ?")
            .bind(id.0)
            .fetch_all(&db.pool)
            .await?;
    for b in &f.work_blocks {
        if !blocks_have.contains(&(b.start, b.end)) {
            blocks::create_block(db, id, b.start, b.end).await?;
        }
    }
    Ok(())
}

fn empty() -> Applied {
    Applied {
        created: 0,
        warnings: vec![],
    }
}
