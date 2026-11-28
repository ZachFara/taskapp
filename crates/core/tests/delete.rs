use taskapp_core::{
    ChangeSource, Db, Error, IssueId, IssuePatch, NewIssue, PinKind, Relation, RelationKind,
    comments, delete, focus, history, issues, labels, links, next_actions, pins, projects,
    relations, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn issue(db: &Db, title: &str) -> Result<IssueId, Error> {
    let new = NewIssue {
        title: title.into(),
        ..NewIssue::default()
    };
    Ok(issues::create(db, new, UI).await?.id)
}

async fn count(db: &Db, sql: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(sql).fetch_one(&db.pool).await
}

/// Tables that point at an issue, with the column that does it.
const REFS: &[(&str, &str)] = &[
    ("comments", "issue_id"),
    ("issue_labels", "issue_id"),
    ("relations", "from_id"),
    ("relations", "to_id"),
    ("links", "issue_id"),
    ("focus_sessions", "issue_id"),
    ("work_blocks", "issue_id"),
    ("event_issues", "issue_id"),
    ("decision_issues", "issue_id"),
    ("import_log", "issue_id"),
];

async fn refs(db: &Db, id: i64) -> Result<i64, sqlx::Error> {
    let mut n = 0;
    for (t, c) in REFS {
        n += count(db, &format!("SELECT COUNT(*) FROM {t} WHERE {c} = {id}")).await?;
    }
    n += count(
        db,
        &format!("SELECT COUNT(*) FROM pins WHERE kind = 'issue' AND target LIKE '%-{id}'"),
    )
    .await?;
    n += count(
        db,
        &format!("SELECT COUNT(*) FROM projects WHERE next_issue_id = {id}"),
    )
    .await?;
    n += count(
        db,
        &format!("SELECT COUNT(*) FROM issues WHERE parent_id = {id}"),
    )
    .await?;
    Ok(n)
}

async fn busy(db: &Db) -> Result<(IssueId, IssueId, IssueId), Box<dyn std::error::Error>> {
    let a = issue(db, "Doomed").await?;
    let b = issue(db, "Other").await?;
    let kid = issues::create(
        db,
        NewIssue {
            title: "Kid".into(),
            parent_id: Some(a),
            ..NewIssue::default()
        },
        UI,
    )
    .await?
    .id;
    let l = labels::create(db, "bug", "red", None, UI).await?;
    issues::update(
        db,
        a,
        IssuePatch {
            label_ids: Some(vec![l.id]),
            ..IssuePatch::default()
        },
        UI,
    )
    .await?;
    comments::add_comment(db, a, "needle remark", None, UI).await?;
    links::add(db, a, "https://example.com/x", None, None, UI).await?;
    more_rows(db, a, b).await?;
    raw_rows(db, a).await?;
    Ok((a, b, kid))
}

/// Relations, focus, a project with its next action, and a pin.
async fn more_rows(db: &Db, a: IssueId, b: IssueId) -> Result<(), Box<dyn std::error::Error>> {
    relations::add(
        db,
        Relation {
            from: a,
            to: b,
            kind: RelationKind::new("blocks"),
        },
        UI,
    )
    .await?;
    relations::add(
        db,
        Relation {
            from: b,
            to: a,
            kind: RelationKind::new("related"),
        },
        UI,
    )
    .await?;
    focus::start(db, a, None, UI).await?;
    let p = projects::create(db, "app", "App", None, None, UI).await?;
    issues::update(
        db,
        a,
        IssuePatch {
            project_id: Some(Some(p.id)),
            ..IssuePatch::default()
        },
        UI,
    )
    .await?;
    next_actions::set(db, p.id, Some(a), UI).await?;
    pins::set(db, PinKind::Issue, &[format!("ABC-{}", a.0)], true).await?;
    Ok(())
}

/// Rows the public API cannot make in one line each.
async fn raw_rows(db: &Db, a: IssueId) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO work_blocks (issue_id, start_utc, end_utc) VALUES (?, 'a', 'b')")
        .bind(a.0)
        .execute(&db.pool)
        .await?;
    sqlx::query(
        "INSERT INTO events (title, starts_at, all_day) VALUES ('e', '2026-01-01T00:00:00', 0)",
    )
    .execute(&db.pool)
    .await?;
    sqlx::query("INSERT INTO event_issues (event_id, issue_id) SELECT id, ? FROM events")
        .bind(a.0)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO decisions (question, options, mode, source) VALUES ('q', '[]', 'assume', 'ui')").execute(&db.pool).await?;
    sqlx::query("INSERT INTO decision_issues (decision_id, issue_id) SELECT id, ? FROM decisions")
        .bind(a.0)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO import_log (external_id, source, issue_id, imported_at) VALUES ('x', 's', ?, 'now')").bind(a.0).execute(&db.pool).await?;
    Ok(())
}

#[tokio::test]
async fn delete_cleans_every_related_table() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b, kid) = busy(&db).await?;
    assert!(refs(&db, a.0).await? >= 12);
    let n = history::step("Deleted", delete::remove(&db, &[a], UI)).await?;
    assert_eq!(n, 1);
    assert_eq!(refs(&db, a.0).await?, 0);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM issues WHERE id = 1").await?,
        0
    );
    assert!(matches!(issues::get(&db, a).await, Err(Error::NotFound(_))));
    assert!(issues::get(&db, b).await.is_ok());
    assert_eq!(issues::get(&db, kid).await?.parent_id, None);
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM issue_fts WHERE issue_fts MATCH 'needle'"
        )
        .await?,
        0
    );
    Ok(())
}

#[tokio::test]
async fn undo_restores_the_issue_and_everything_it_owned() -> R {
    let db = Db::open_in_memory().await?;
    let (a, _, kid) = busy(&db).await?;
    let before = refs(&db, a.0).await?;
    history::step("Deleted", delete::remove(&db, &[a], UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(refs(&db, a.0).await?, before);
    let i = issues::get(&db, a).await?;
    assert_eq!((i.title.as_str(), i.label_ids.len()), ("Doomed", 1));
    assert_eq!(issues::get(&db, kid).await?.parent_id, Some(a));
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM issue_fts WHERE issue_fts MATCH 'needle'"
        )
        .await?,
        1
    );
    undo::redo(&db).await?;
    assert_eq!(refs(&db, a.0).await?, 0);
    Ok(())
}

#[tokio::test]
async fn several_issues_one_step_and_unknown_deletes_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b, kid) = (
        issue(&db, "A").await?,
        issue(&db, "B").await?,
        issue(&db, "C").await?,
    );
    let r = history::step("Deleted", delete::remove(&db, &[a, b, IssueId(999)], UI)).await;
    assert!(matches!(r, Err(Error::NotFound(_))));
    assert!(issues::get(&db, a).await.is_ok());
    assert_eq!(
        history::step("Deleted", delete::remove(&db, &[a, b], UI)).await?,
        2
    );
    assert!(issues::get(&db, kid).await.is_ok());
    undo::undo(&db).await?;
    assert!(issues::get(&db, a).await.is_ok() && issues::get(&db, b).await.is_ok());
    Ok(())
}

#[tokio::test]
async fn empty_archive_deletes_only_archived() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b) = (issue(&db, "Old").await?, issue(&db, "Live").await?);
    issues::archive(&db, a, UI).await?;
    assert_eq!(
        history::step("Emptied", delete::empty_archive(&db, UI)).await?,
        1
    );
    assert!(matches!(issues::get(&db, a).await, Err(Error::NotFound(_))));
    assert!(issues::get(&db, b).await.is_ok());
    assert_eq!(
        history::step("Emptied", delete::empty_archive(&db, UI)).await?,
        0
    );
    Ok(())
}
