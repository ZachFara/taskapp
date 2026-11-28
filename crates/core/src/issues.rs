use chrono::TimeZone as _;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::{Map, Value};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Depth, Due, Error, Issue, IssueId, IssuePatch, LabelId, NewIssue, Relation,
    RelationKind, Result, StatusCategory, StatusId, checklist, history, recur::Rule, relations,
    search, statuses::parse_category,
};

fn bad<E: std::fmt::Display>(e: E) -> Error {
    Error::Invalid(e.to_string())
}

pub(crate) async fn prefix(conn: &mut SqliteConnection) -> Result<String> {
    let v = sqlx::query_scalar!("SELECT value FROM settings WHERE key = 'issue_prefix'")
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| Error::Invalid("issue_prefix setting is missing".into()))?;
    serde_json::from_str(&v).map_err(bad)
}

async fn category(conn: &mut SqliteConnection, id: StatusId) -> Result<StatusCategory> {
    let c = sqlx::query_scalar!("SELECT category FROM statuses WHERE id = ?", id.0)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| Error::Invalid(format!("status {} does not exist", id.0)))?;
    parse_category(&c)
}

fn is_done(c: StatusCategory) -> bool {
    matches!(c, StatusCategory::Completed | StatusCategory::Canceled)
}

pub(crate) async fn fetch(conn: &mut SqliteConnection, id: IssueId) -> Result<Issue> {
    let r = sqlx::query!(
        r#"SELECT id AS "id!: i64", number, title, description, status_id, priority,
                  project_id, milestone_id, parent_id, cycle_id,
                  start_date AS "start_date: NaiveDate", due_date AS "due_date: NaiveDate",
                  due_time_utc AS "due_time_utc: DateTime<Utc>", due_tz, estimate_minutes, points, depth, repeat_rule, properties,
                  hidden_until AS "hidden_until: NaiveDate",
                  created_at AS "created_at: DateTime<Utc>", updated_at AS "updated_at: DateTime<Utc>",
                  completed_at AS "completed_at: DateTime<Utc>", archived_at AS "archived_at: DateTime<Utc>"
           FROM issues WHERE id = ?"#,
        id.0
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| Error::NotFound(format!("issue {}", id.0)))?;
    let label_ids = sqlx::query_scalar!(
        r#"SELECT label_id AS "label_id!: i64" FROM issue_labels WHERE issue_id = ? ORDER BY label_id"#,
        id.0
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(LabelId)
    .collect();
    let due = match (r.due_time_utc, r.due_tz, r.due_date) {
        (Some(utc), Some(tz), _) => Some(Due::DateTime { utc, tz }),
        (_, _, Some(d)) => Some(Due::Date(d)),
        _ => None,
    };
    Ok(Issue {
        id,
        key: format!("{}-{}", prefix(conn).await?, r.number),
        title: r.title,
        description: r.description,
        status_id: StatusId(r.status_id),
        priority: u8::try_from(r.priority).map_err(bad)?,
        project_id: r.project_id.map(crate::ProjectId),
        milestone_id: r.milestone_id.map(crate::MilestoneId),
        parent_id: r.parent_id.map(IssueId),
        cycle_id: r.cycle_id.map(crate::CycleId),
        label_ids,
        start_date: r.start_date,
        due,
        estimate_minutes: r
            .estimate_minutes
            .map(u32::try_from)
            .transpose()
            .map_err(bad)?,
        points: r.points,
        hidden_until: r.hidden_until,
        depth: r.depth.as_deref().map(parse_depth).transpose()?,
        repeat: r.repeat_rule,
        properties: serde_json::from_str(&r.properties).map_err(bad)?,
        created_at: r.created_at,
        updated_at: r.updated_at,
        completed_at: r.completed_at,
        archived_at: r.archived_at,
    })
}

/// The stored word for a depth; anything else in the column is a bug in the writer.
fn parse_depth(s: &str) -> Result<Depth> {
    match s {
        "deep" => Ok(Depth::Deep),
        "shallow" => Ok(Depth::Shallow),
        other => Err(Error::Invalid(format!(
            "issue depth {other} is not deep or shallow"
        ))),
    }
}

/// Columns every issue query selects: the row plus its label ids as a comma list.
pub(crate) const ISSUE_COLS: &str = "issues.*, \
    (SELECT group_concat(label_id) FROM issue_labels WHERE issue_id = issues.id) AS label_ids";

pub(crate) fn issue_from_row(r: &SqliteRow, prefix: &str) -> Result<Issue> {
    let id: i64 = r.try_get("id")?;
    let due = match (
        r.try_get::<Option<DateTime<Utc>>, _>("due_time_utc")?,
        r.try_get::<Option<String>, _>("due_tz")?,
        r.try_get::<Option<NaiveDate>, _>("due_date")?,
    ) {
        (Some(utc), Some(tz), _) => Some(Due::DateTime { utc, tz }),
        (_, _, Some(d)) => Some(Due::Date(d)),
        _ => None,
    };
    let props: String = r.try_get("properties")?;
    let labels: Option<String> = r.try_get("label_ids")?;
    let opt = |c: &str| -> Result<Option<i64>> { Ok(r.try_get(c)?) };
    Ok(Issue {
        id: IssueId(id),
        key: format!("{prefix}-{}", r.try_get::<i64, _>("number")?),
        title: r.try_get("title")?,
        description: r.try_get("description")?,
        status_id: StatusId(r.try_get("status_id")?),
        priority: u8::try_from(r.try_get::<i64, _>("priority")?)
            .map_err(|e| Error::Invalid(e.to_string()))?,
        project_id: opt("project_id")?.map(crate::ProjectId),
        milestone_id: opt("milestone_id")?.map(crate::MilestoneId),
        parent_id: opt("parent_id")?.map(IssueId),
        cycle_id: opt("cycle_id")?.map(crate::CycleId),
        label_ids: labels
            .iter()
            .flat_map(|s| s.split(','))
            .filter_map(|s| s.parse().ok().map(LabelId))
            .collect(),
        start_date: r.try_get("start_date")?,
        due,
        estimate_minutes: opt("estimate_minutes")?
            .map(|m| u32::try_from(m).map_err(|e| Error::Invalid(e.to_string())))
            .transpose()?,
        points: r.try_get("points")?,
        hidden_until: r.try_get("hidden_until")?,
        depth: r
            .try_get::<Option<String>, _>("depth")?
            .as_deref()
            .map(parse_depth)
            .transpose()?,
        repeat: r.try_get("repeat_rule")?,
        properties: serde_json::from_str::<Map<String, Value>>(&props)
            .map_err(|e| Error::Invalid(format!("issue properties: {e}")))?,
        created_at: r.try_get("created_at")?,
        updated_at: r.try_get("updated_at")?,
        completed_at: r.try_get("completed_at")?,
        archived_at: r.try_get("archived_at")?,
    })
}

/// Rows from a query that selected [`ISSUE_COLS`].
pub(crate) async fn issues_from_rows(
    conn: &mut SqliteConnection,
    rows: &[SqliteRow],
) -> Result<Vec<Issue>> {
    let prefix = prefix(conn).await?;
    rows.iter().map(|r| issue_from_row(r, &prefix)).collect()
}

/// 0 to 1000 with at most one decimal, judged on the shortest text of the number so a float's noise
/// (0.30000000000000004) is refused rather than rounded.
pub(crate) fn valid_points(p: f64) -> bool {
    let one_decimal = |s: String| s.split_once('.').is_none_or(|(_, f)| f.len() == 1);
    p.is_finite() && (0.0..=1000.0).contains(&p) && one_decimal(p.to_string())
}

fn check_points(points: Option<f64>) -> Result<()> {
    let Some(p) = points else { return Ok(()) };
    if !valid_points(p) {
        return Err(Error::Invalid(format!(
            "points {p} must be 0 to 1000 with at most one decimal"
        )));
    }
    Ok(())
}

fn check_fields(title: &str, priority: u8, points: Option<f64>) -> Result<()> {
    check_points(points)?;
    if title.trim().is_empty() {
        return Err(Error::Invalid("title is empty".into()));
    }
    if priority > 4 {
        return Err(Error::Invalid(format!("priority {priority} is not 0 to 4")));
    }
    Ok(())
}

struct DueCols<'a> {
    date: Option<NaiveDate>,
    utc: Option<DateTime<Utc>>,
    tz: Option<&'a str>,
}

fn due_cols(due: Option<&Due>) -> DueCols<'_> {
    let (mut date, mut utc, mut tz) = (None, None, None);
    match due {
        Some(Due::Date(d)) => date = Some(*d),
        Some(Due::DateTime { utc: u, tz: z }) => {
            (utc, tz) = (Some(*u), Some(z.as_str()));
            // Views filter and sort on due_date, so timed dues carry their local date there too.
            date = Some(z.parse::<chrono_tz::Tz>().map_or_else(
                |_| u.date_naive(),
                |zone| u.with_timezone(&zone).date_naive(),
            ));
        }
        None => {}
    }
    DueCols { date, utc, tz }
}

async fn set_labels(conn: &mut SqliteConnection, id: IssueId, labels: &[LabelId]) -> Result<()> {
    sqlx::query!("DELETE FROM issue_labels WHERE issue_id = ?", id.0)
        .execute(&mut *conn)
        .await?;
    for l in labels {
        sqlx::query!(
            "INSERT OR IGNORE INTO issue_labels (issue_id, label_id) VALUES (?, ?)",
            id.0,
            l.0
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn record_created(tx: &mut SqliteConnection, src: ChangeSource, issue: &Issue) -> Result<()> {
    let id = issue.id.0.to_string();
    let (title, key) = (
        Value::from(issue.title.as_str()),
        Value::from(issue.key.as_str()),
    );
    // What the new issue points at is recorded too, so undoing the create lets those rows be deleted, and a
    // redo links them again. The key comes last: undo archives first, redo restores last.
    let refs = [
        ("project_id", issue.project_id.map(|p| p.0)),
        ("milestone_id", issue.milestone_id.map(|m| m.0)),
        ("cycle_id", issue.cycle_id.map(|c| c.0)),
    ]
    .into_iter()
    .map(|(f, v)| (f, v.map(Value::from)))
    .chain(
        issue
            .label_ids
            .iter()
            .map(|l| ("label", Some(Value::from(l.0)))),
    );
    let fields = [("title", Some(title))]
        .into_iter()
        .chain(refs)
        .chain([("key", Some(key))]);
    for (field, v) in fields {
        let edit = history::Edit {
            field,
            old: None,
            new: v.as_ref(),
        };
        if v.is_some() {
            history::record(tx, src, "issue", &id, &edit).await?;
        }
    }
    search::reindex_in(tx, issue.id).await
}

/// # Errors
///
/// `Invalid` for an empty title, a priority above 4 or an unknown status; `Db` for other failures.
pub async fn create(db: &Db, new: NewIssue, src: ChangeSource) -> Result<Issue> {
    // IMMEDIATE takes the write lock up front, so two writers cannot read the same max(number).
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let issue = create_in(&mut tx, new, src).await?;
    tx.commit().await?;
    Ok(issue)
}

/// The workspace's chosen starting status, else the first one (also when the chosen one was deleted).
async fn starting_status(tx: &mut SqliteConnection) -> Result<StatusId> {
    let id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM statuses ORDER BY id = (SELECT CAST(json_extract(value, '$') AS INTEGER) \
         FROM settings WHERE key = 'default_status') DESC, position, id LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    id.map(StatusId)
        .ok_or_else(|| Error::Invalid("no statuses exist".into()))
}
/// [`create`] inside the caller's transaction, which should be `BEGIN IMMEDIATE`.
///
/// # Errors
///
/// As [`create`].
pub async fn create_in(
    tx: &mut SqliteConnection,
    new: NewIssue,
    src: ChangeSource,
) -> Result<Issue> {
    let priority = new.priority.unwrap_or(0);
    check_fields(&new.title, priority, new.points)?;
    check_props(tx, None, new.properties.as_ref()).await?;
    let status = match new.status_id {
        Some(s) => s,
        None => starting_status(tx).await?,
    };
    let now = Utc::now();
    let created = new.created_at.unwrap_or(now);
    let completed = match new.completed_at {
        Some(c) => Some(c),
        None => is_done(category(tx, status).await?).then_some(now),
    };
    let due = due_cols(new.due.as_ref());
    let props = serde_json::to_string(&new.properties.unwrap_or_default()).map_err(bad)?;
    let description = new.description.unwrap_or_default();
    let (project, milestone) = (new.project_id.map(|p| p.0), new.milestone_id.map(|m| m.0));
    let (parent, cycle) = (new.parent_id.map(|p| p.0), new.cycle_id.map(|c| c.0));
    let id = sqlx::query!(
        "INSERT INTO issues (number, title, description, status_id, priority, project_id, milestone_id,
           parent_id, cycle_id, start_date, due_date, due_time_utc, due_tz, estimate_minutes, points, properties,
           created_at, updated_at, completed_at)
         VALUES ((SELECT COALESCE(MAX(number), 0) + 1 FROM issues), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
           ?, ?, ?, ?)",
        new.title,
        description,
        status.0,
        priority,
        project,
        milestone,
        parent,
        cycle,
        new.start_date,
        due.date,
        due.utc,
        due.tz,
        new.estimate_minutes,
        new.points,
        props,
        created,
        created,
        completed
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    let id = IssueId(id);
    set_labels(tx, id, &new.label_ids.unwrap_or_default()).await?;
    let issue = fetch(tx, id).await?;
    record_created(tx, src, &issue).await?;
    Ok(issue)
}

/// Collects `(field, old, new)` when the two values differ.
fn diff<T: Serialize + PartialEq>(
    out: &mut Vec<(&'static str, Value, Value)>,
    field: &'static str,
    old: &T,
    new: &T,
) {
    if old != new {
        out.push((
            field,
            serde_json::to_value(old).unwrap_or(Value::Null),
            serde_json::to_value(new).unwrap_or(Value::Null),
        ));
    }
}

fn apply_patch(old: &Issue, p: IssuePatch) -> Issue {
    let mut n = old.clone();
    n.title = p.title.unwrap_or(n.title);
    n.description = p.description.unwrap_or(n.description);
    n.status_id = p.status_id.unwrap_or(n.status_id);
    n.priority = p.priority.unwrap_or(n.priority);
    n.project_id = p.project_id.unwrap_or(n.project_id);
    n.milestone_id = p.milestone_id.unwrap_or(n.milestone_id);
    n.parent_id = p.parent_id.unwrap_or(n.parent_id);
    n.cycle_id = p.cycle_id.unwrap_or(n.cycle_id);
    n.start_date = p.start_date.unwrap_or(n.start_date);
    n.due = p.due.unwrap_or(n.due);
    n.estimate_minutes = p.estimate_minutes.unwrap_or(n.estimate_minutes);
    n.points = p.points.unwrap_or(n.points);
    n.hidden_until = p.hidden_until.unwrap_or(n.hidden_until);
    n.depth = p.depth.unwrap_or(n.depth);
    n.repeat = p.repeat.unwrap_or(n.repeat);
    if let Some(mut l) = p.label_ids {
        l.sort_by_key(|l| l.0);
        l.dedup();
        n.label_ids = l;
    }
    for (k, v) in p.properties.unwrap_or_default() {
        if v.is_null() {
            n.properties.remove(&k);
        } else {
            n.properties.insert(k, v);
        }
    }
    n
}

/// Keys with a definition must hold a valid value; keys without one pass (migrated data).
async fn check_props(
    conn: &mut SqliteConnection,
    old: Option<&Map<String, Value>>,
    new: Option<&Map<String, Value>>,
) -> Result<()> {
    let changed = new
        .into_iter()
        .flatten()
        .filter(|(k, v)| old.and_then(|o| o.get(*k)) != Some(*v));
    for (k, v) in changed {
        match crate::properties::get_def(conn, k).await {
            Ok(def) => crate::properties::validate(&def, v)?,
            Err(Error::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Walking up from the new parent must not reach the issue (a self-parent is the shortest cycle).
async fn check_parent(
    conn: &mut SqliteConnection,
    id: IssueId,
    parent: Option<IssueId>,
) -> Result<()> {
    let Some(parent) = parent else { return Ok(()) };
    let cyclic = sqlx::query_scalar::<_, i64>(
        "WITH RECURSIVE up(id) AS (SELECT ?1 UNION SELECT i.parent_id FROM issues i JOIN up ON i.id = up.id
             WHERE i.parent_id IS NOT NULL)
         SELECT EXISTS (SELECT 1 FROM up WHERE id = ?2)",
    )
    .bind(parent.0)
    .bind(id.0)
    .fetch_one(conn)
    .await?;
    if cyclic == 0 {
        Ok(())
    } else {
        Err(Error::Invalid("parent would create a cycle".into()))
    }
}

fn changes(old: &Issue, n: &Issue) -> Vec<(&'static str, Value, Value)> {
    let mut c = Vec::new();
    diff(&mut c, "title", &old.title, &n.title);
    diff(&mut c, "description", &old.description, &n.description);
    diff(&mut c, "status_id", &old.status_id, &n.status_id);
    diff(&mut c, "priority", &old.priority, &n.priority);
    diff(&mut c, "project_id", &old.project_id, &n.project_id);
    diff(&mut c, "milestone_id", &old.milestone_id, &n.milestone_id);
    diff(&mut c, "parent_id", &old.parent_id, &n.parent_id);
    diff(&mut c, "cycle_id", &old.cycle_id, &n.cycle_id);
    diff(&mut c, "label_ids", &old.label_ids, &n.label_ids);
    diff(&mut c, "start_date", &old.start_date, &n.start_date);
    diff(&mut c, "due", &old.due, &n.due);
    diff(
        &mut c,
        "estimate_minutes",
        &old.estimate_minutes,
        &n.estimate_minutes,
    );
    diff(&mut c, "points", &old.points, &n.points);
    diff(&mut c, "hidden_until", &old.hidden_until, &n.hidden_until);
    diff(&mut c, "depth", &old.depth, &n.depth);
    diff(&mut c, "repeat", &old.repeat, &n.repeat);
    diff(&mut c, "properties", &old.properties, &n.properties);
    c
}

async fn record_all(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    id: IssueId,
    cs: &[(&str, Value, Value)],
) -> Result<()> {
    for (field, old, new) in cs {
        let edit = history::Edit {
            field,
            old: (!old.is_null()).then_some(old),
            new: (!new.is_null()).then_some(new),
        };
        history::record(tx, src, "issue", &id.0.to_string(), &edit).await?;
    }
    if cs.iter().any(|c| matches!(c.0, "title" | "description")) {
        search::reindex_in(tx, id).await?;
    }
    Ok(())
}

/// A patch that changes nothing writes nothing, not even `updated_at`.
///
/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for an empty title, a priority above 4, an unknown
/// status or a parent that would close a cycle.
pub async fn update(db: &Db, id: IssueId, patch: IssuePatch, src: ChangeSource) -> Result<Issue> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let issue = update_in(&mut tx, id, patch, src).await?;
    tx.commit().await?;
    Ok(issue)
}

/// The completion time after a move to `status`: kept when the issue was already done, else now, or none.
async fn completed_after(
    tx: &mut SqliteConnection,
    old: &Issue,
    status: StatusId,
) -> Result<Option<DateTime<Utc>>> {
    let was = is_done(category(tx, old.status_id).await?);
    Ok(if is_done(category(tx, status).await?) {
        if was {
            old.completed_at
        } else {
            Some(Utc::now())
        }
    } else {
        None
    })
}

/// [`update`] inside the caller's transaction, which should be `BEGIN IMMEDIATE`.
///
/// # Errors
///
/// As [`update`].
pub async fn update_in(
    tx: &mut SqliteConnection,
    id: IssueId,
    patch: IssuePatch,
    src: ChangeSource,
) -> Result<Issue> {
    write_in(tx, id, patch, src, true).await
}

/// [`update_in`] for undo and redo: finishing a repeating issue then creates nothing, because the
/// recorded edits of the original step (the cleared rule, the new issue) replay on their own.
pub(crate) async fn restore_in(
    tx: &mut SqliteConnection,
    id: IssueId,
    patch: IssuePatch,
    src: ChangeSource,
) -> Result<Issue> {
    write_in(tx, id, patch, src, false).await
}

/// The workspace time zone, read inside the caller's transaction.
async fn zone_in(tx: &mut SqliteConnection) -> Result<chrono_tz::Tz> {
    let name = sqlx::query_scalar::<_, String>(
        "SELECT json_extract(value, '$') FROM settings WHERE key = 'timezone'",
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or_else(crate::workspace::system_tz);
    name.parse()
        .map_err(|_| Error::Invalid(format!("unknown time zone {name}")))
}

/// `due` moved to the local day `next`, keeping a timed due's time of day; a missing due becomes that date.
fn due_on(due: Option<&Due>, next: NaiveDate, zone: chrono_tz::Tz) -> Due {
    let Some(Due::DateTime { utc, tz }) = due else {
        return Due::Date(next);
    };
    let z: chrono_tz::Tz = tz.parse().unwrap_or(zone);
    z.from_local_datetime(&next.and_time(utc.with_timezone(&z).time()))
        .earliest()
        .map_or(Due::Date(next), |t| Due::DateTime {
            utc: t.with_timezone(&Utc),
            tz: tz.clone(),
        })
}

/// Skips this occurrence of a repeating issue: its due date moves to the next one, nothing is created.
///
/// # Errors
/// `Invalid` when the issue does not repeat; `NotFound` for an unknown id.
pub async fn skip_repeat(db: &Db, id: IssueId, src: ChangeSource) -> Result<Issue> {
    let issue = get(db, id).await?;
    let rule: Rule = issue
        .repeat
        .as_deref()
        .ok_or_else(|| Error::Invalid(format!("{} does not repeat", issue.key)))?
        .parse()?;
    let zone = crate::insights::zone(db).await?;
    let today = crate::insights::local(Utc::now(), zone);
    let next = rule.next(due_cols(issue.due.as_ref()).date.unwrap_or(today), today);
    let due = due_on(issue.due.as_ref(), next, zone);
    update(
        db,
        id,
        IssuePatch {
            due: Some(Some(due)),
            ..IssuePatch::default()
        },
        src,
    )
    .await
}

/// Creates the next occurrence of a repeating issue that was just finished: a copy with the ticks
/// cleared, the next due date and the same rule, linked to it with a `follows` relation.
async fn spawn_next(
    tx: &mut SqliteConnection,
    done: &Issue,
    rule: &Rule,
    src: ChangeSource,
) -> Result<()> {
    let zone = zone_in(tx).await?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    let base = due_cols(done.due.as_ref()).date.unwrap_or(today);
    let next = rule.next(base, today);
    let shift = next - base;
    let new = NewIssue {
        title: done.title.clone(),
        description: Some(checklist::reset(&done.description)),
        priority: Some(done.priority),
        project_id: done.project_id,
        milestone_id: done.milestone_id,
        parent_id: done.parent_id,
        label_ids: Some(done.label_ids.clone()),
        start_date: done.start_date.map(|s| s + shift),
        due: Some(due_on(done.due.as_ref(), next, zone)),
        estimate_minutes: done.estimate_minutes,
        points: done.points,
        properties: Some(done.properties.clone()),
        ..NewIssue::default()
    };
    let made = create_in(tx, new, src).await?;
    // Not recorded as edits: undoing the step archives the new issue, which carries these along.
    sqlx::query("UPDATE issues SET depth = ?, repeat_rule = ? WHERE id = ?")
        .bind(done.depth.map(Depth::as_str))
        .bind(rule.to_string())
        .bind(made.id.0)
        .execute(&mut *tx)
        .await?;
    let rel = Relation {
        from: made.id,
        to: done.id,
        kind: RelationKind("follows".into()),
    };
    relations::add_in(tx, rel, src).await
}

/// Checks and canonicalises a changed repeat rule. Finishing (not cancelling) a repeating issue hands the rule to
/// the next occurrence: the rule is cleared on `new` and returned.
async fn settle_repeat(
    tx: &mut SqliteConnection,
    old: &Issue,
    new: &mut Issue,
    spawn: bool,
) -> Result<Option<Rule>> {
    if new.repeat != old.repeat {
        new.repeat = new
            .repeat
            .as_deref()
            .map(|r| r.parse::<Rule>().map(|r| r.to_string()))
            .transpose()?;
    }
    let (Some(rule), true) = (&old.repeat, spawn && new.status_id != old.status_id) else {
        return Ok(None);
    };
    if new.repeat != old.repeat {
        return Ok(None);
    }
    let done = category(tx, new.status_id).await? == StatusCategory::Completed;
    let was = is_done(category(tx, old.status_id).await?);
    if !done || was {
        return Ok(None);
    }
    new.repeat = None;
    Ok(Some(rule.parse()?))
}

async fn write_in(
    tx: &mut SqliteConnection,
    id: IssueId,
    patch: IssuePatch,
    src: ChangeSource,
    spawn: bool,
) -> Result<Issue> {
    let old = fetch(tx, id).await?;
    let mut new = apply_patch(&old, patch);
    check_fields(&new.title, new.priority, new.points)?;
    check_parent(tx, id, new.parent_id).await?;
    check_props(tx, Some(&old.properties), Some(&new.properties)).await?;
    let finished = settle_repeat(tx, &old, &mut new, spawn).await?;
    let cs = changes(&old, &new);
    if cs.is_empty() {
        return Ok(old);
    }
    if new.status_id != old.status_id {
        new.completed_at = completed_after(tx, &old, new.status_id).await?;
    }
    new.updated_at = Utc::now();
    let due = due_cols(new.due.as_ref());
    let props = serde_json::to_string(&new.properties).map_err(bad)?;
    let depth = new.depth.map(Depth::as_str);
    let (project, milestone) = (new.project_id.map(|p| p.0), new.milestone_id.map(|m| m.0));
    let (parent, cycle) = (new.parent_id.map(|p| p.0), new.cycle_id.map(|c| c.0));
    sqlx::query!(
        "UPDATE issues SET title = ?, description = ?, status_id = ?, priority = ?, project_id = ?,
           milestone_id = ?, parent_id = ?, cycle_id = ?, start_date = ?, due_date = ?, due_time_utc = ?,
           due_tz = ?, estimate_minutes = ?, points = ?, hidden_until = ?, depth = ?, repeat_rule = ?, properties = ?, updated_at = ?, completed_at = ? WHERE id = ?",
        new.title,
        new.description,
        new.status_id.0,
        new.priority,
        project,
        milestone,
        parent,
        cycle,
        new.start_date,
        due.date,
        due.utc,
        due.tz,
        new.estimate_minutes,
        new.points,
        new.hidden_until,
        depth,
        new.repeat,
        props,
        new.updated_at,
        new.completed_at,
        id.0
    )
    .execute(&mut *tx)
    .await?;
    if new.label_ids != old.label_ids {
        set_labels(tx, id, &new.label_ids).await?;
    }
    record_all(tx, src, id, &cs).await?;
    if let Some(rule) = finished {
        spawn_next(tx, &new, &rule, src).await?;
    }
    Ok(new)
}

/// # Errors
///
/// `NotFound` for an unknown id.
pub async fn get(db: &Db, id: IssueId) -> Result<Issue> {
    fetch(&mut *db.pool.acquire().await?, id).await
}

/// # Errors
///
/// `NotFound` when the key is malformed or names no issue. The prefix is display-only: the number
/// alone identifies the issue, so keys from before a prefix change still resolve.
pub async fn get_by_key(db: &Db, key: &str) -> Result<Issue> {
    let mut conn = db.pool.acquire().await?;
    let not_found = || Error::NotFound(format!("issue {key}"));
    let key = key.trim().to_ascii_uppercase();
    let (p, n) = key.rsplit_once('-').ok_or_else(not_found)?;
    // Only the digits the display shows: no sign, no leading zero.
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) || n.starts_with('0') {
        return Err(not_found());
    }
    let number: i64 = n.parse().map_err(|_| not_found())?;
    crate::workspace::validate_key(p).map_err(|_| not_found())?;
    let id = sqlx::query_scalar!(
        r#"SELECT id AS "id!: i64" FROM issues WHERE number = ?"#,
        number
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(not_found)?;
    fetch(&mut conn, IssueId(id)).await
}

async fn set_archived(db: &Db, id: IssueId, src: ChangeSource, archive: bool) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    set_archived_in(&mut tx, id, src, archive).await?;
    tx.commit().await?;
    Ok(())
}

/// Archives or restores inside the caller's transaction; already in that state is a no-op.
pub(crate) async fn set_archived_in(
    tx: &mut SqliteConnection,
    id: IssueId,
    src: ChangeSource,
    archive: bool,
) -> Result<()> {
    let issue = fetch(tx, id).await?;
    if issue.archived_at.is_some() == archive {
        return Ok(());
    }
    let at = archive.then(Utc::now);
    sqlx::query!("UPDATE issues SET archived_at = ? WHERE id = ?", at, id.0)
        .execute(&mut *tx)
        .await?;
    let (old, new) = (
        issue
            .archived_at
            .map_or(Value::Null, |t| Value::from(t.to_rfc3339())),
        at.map_or(Value::Null, |t| Value::from(t.to_rfc3339())),
    );
    record_all(tx, src, id, &[("archived_at", old, new)]).await
}

/// Archiving an archived issue is a no-op.
///
/// # Errors
///
/// `NotFound` for an unknown id.
pub async fn archive(db: &Db, id: IssueId, src: ChangeSource) -> Result<()> {
    set_archived(db, id, src, true).await
}

/// Restoring an issue that is not archived is a no-op.
///
/// # Errors
///
/// `NotFound` for an unknown id.
pub async fn restore(db: &Db, id: IssueId, src: ChangeSource) -> Result<()> {
    set_archived(db, id, src, false).await
}

/// Non-archived sub-issues, oldest number first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn children(db: &Db, id: IssueId) -> Result<Vec<Issue>> {
    let mut conn = db.pool.acquire().await?;
    let ids = sqlx::query_scalar!(
        r#"SELECT id AS "id!: i64" FROM issues WHERE parent_id = ? AND archived_at IS NULL ORDER BY number"#,
        id.0
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::with_capacity(ids.len());
    for c in ids {
        out.push(fetch(&mut conn, IssueId(c)).await?);
    }
    Ok(out)
}
