use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::json;
use taskapp_core::{
    ChangeSource, Db, Due, IssueId, Layout, NewIssue, PinKind, PropertyDef, PropertyKind, Relation,
    RelationKind, StatusCategory, ViewDef, comments, cycles, issues, labels, pins, projects,
    properties, relation_kinds, relations, settings, views,
};
use taskapp_export::{git, rebuild, write};

type R = Result<(), Box<dyn std::error::Error>>;

const SRC: ChangeSource = ChangeSource::Ui;

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

/// A database with one of everything the export writes.
async fn seeded() -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    let (docs, ship) = seed_issues(&db).await?;
    seed_rest(&db, docs, ship).await?;
    Ok(db)
}

/// Projects, cycles, labels, two linked issues, a relation and a comment.
async fn seed_issues(db: &Db) -> Result<(IssueId, IssueId), Box<dyn std::error::Error>> {
    let app = projects::create(db, "app", "App", None, None, SRC).await?;
    let v1 = projects::create_milestone(db, app.id, "v1", Some(day(2026, 3, 1)), SRC).await?;
    let sprint = cycles::create(
        db,
        Some("Sprint".into()),
        day(2026, 1, 1),
        day(2026, 1, 14),
        taskapp_core::palette::Look::default(),
        SRC,
    )
    .await?;
    let bug = labels::create(db, "bug", "#ff0000", None, SRC).await?;
    labels::create(db, "backend", "#00ff00", None, SRC).await?;
    let docs = issues::create(
        db,
        NewIssue {
            title: "Write docs".into(),
            description: Some("Line one\n\nLine two".into()),
            project_id: Some(app.id),
            milestone_id: Some(v1.id),
            cycle_id: Some(sprint.id),
            label_ids: Some(vec![bug.id]),
            priority: Some(2),
            due: Some(Due::Date(day(2026, 1, 10))),
            estimate_minutes: Some(90),
            points: Some(3.5),
            ..NewIssue::default()
        },
        SRC,
    )
    .await?;
    let ship = issues::create(
        db,
        NewIssue {
            title: "Ship".into(),
            parent_id: Some(docs.id),
            due: Some(Due::DateTime {
                utc: Utc
                    .with_ymd_and_hms(2026, 1, 12, 14, 0, 0)
                    .single()
                    .ok_or("bad time")?,
                tz: "Europe/Berlin".into(),
            }),
            ..NewIssue::default()
        },
        SRC,
    )
    .await?;
    link_and_comment(db, docs.id, ship.id).await?;
    Ok((docs.id, ship.id))
}

/// A blocking relation between two issues, and a comment on the first.
async fn link_and_comment(db: &Db, docs: IssueId, ship: IssueId) -> R {
    relations::add(
        db,
        Relation {
            from: docs,
            to: ship,
            kind: RelationKind::new("blocks"),
        },
        SRC,
    )
    .await?;
    let at = Utc
        .with_ymd_and_hms(2026, 1, 3, 9, 0, 0)
        .single()
        .ok_or("bad time")?;
    comments::add_comment(db, docs, "first\n### not a heading", Some(at), SRC).await?;
    Ok(())
}

/// Properties with a value, a done issue, a saved view, a pin and a setting.
async fn seed_rest(db: &Db, docs: IssueId, ship: IssueId) -> R {
    properties::create_def(
        db,
        PropertyDef {
            key: "owner".into(),
            name: "Owner".into(),
            kind: PropertyKind::Text,
            options: vec![],
            project_id: None,
            position: 0,
        },
        SRC,
    )
    .await?;
    properties::set_value(db, docs, "owner", Some(json!("sam")), SRC).await?;
    let done = statuses_named(db, "Done").await?;
    issues::update(
        db,
        ship,
        taskapp_core::IssuePatch {
            status_id: Some(done),
            ..Default::default()
        },
        SRC,
    )
    .await?;
    views::save(
        db,
        None,
        "Mine",
        ViewDef {
            filter: None,
            sorts: vec![],
            group_by: None,
            layout: Layout::Board,
            columns: vec![],
            include_archived: false,
            ..ViewDef::default()
        },
        taskapp_core::palette::Look::default(),
        SRC,
    )
    .await?;
    pins::pin(db, PinKind::Url, "https://example.com", Some("Site".into())).await?;
    settings::set(db, "gcal_enabled", &true).await?;
    Ok(())
}

async fn statuses_named(
    db: &Db,
    name: &str,
) -> Result<taskapp_core::StatusId, Box<dyn std::error::Error>> {
    let all = taskapp_core::statuses::list(db).await?;
    let s = all
        .iter()
        .find(|s| s.name == name)
        .ok_or("no such status")?;
    assert_eq!(s.category, StatusCategory::Completed);
    Ok(s.id)
}

/// Every file under `dir` (except `.git`) as a relative path and its bytes.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("test io") {
            let path = entry.expect("test io").path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("test io")
                    .to_string_lossy()
                    .into_owned();
                out.insert(rel, fs::read(&path).expect("test io"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("test io");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("test io")
}

/// A repo with a local identity, so commits work without the machine's git config.
fn temp_repo(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    fs::create_dir_all(&repo).expect("test io");
    git_in(&repo, &["init", "-q"]);
    git_in(&repo, &["config", "user.name", "test"]);
    git_in(&repo, &["config", "user.email", "test@example.invalid"]);
    repo
}

fn commit_count(dir: &Path) -> usize {
    git_in(dir, &["rev-list", "--count", "HEAD"])
        .trim()
        .parse()
        .expect("test io")
}

#[tokio::test]
async fn export_is_deterministic() -> R {
    let db = seeded().await?;
    let (a, b) = (tempfile::tempdir()?, tempfile::tempdir()?);
    write::write_snapshot(&db, a.path()).await?;
    write::write_snapshot(&db, b.path()).await?;
    assert_eq!(tree(a.path()), tree(b.path()));
    let again = write::write_snapshot(&db, a.path()).await?;
    assert_eq!(
        again.files_written, 0,
        "unchanged data must not rewrite files"
    );
    Ok(())
}

#[tokio::test]
async fn rebuild_roundtrip() -> R {
    let db = seeded().await?;
    let (src, dst) = (tempfile::tempdir()?, tempfile::tempdir()?);
    write::write_snapshot(&db, src.path()).await?;
    let rebuilt = rebuild::rebuild(src.path(), &dst.path().join("rebuilt.db")).await?;
    let again = tempfile::tempdir()?;
    write::write_snapshot(&rebuilt, again.path()).await?;
    assert_eq!(tree(src.path()), tree(again.path()));
    Ok(())
}

#[tokio::test]
async fn points_survive_export_and_rebuild_and_stay_out_of_files_without_them() -> R {
    let db = seeded().await?;
    let (src, dst) = (tempfile::tempdir()?, tempfile::tempdir()?);
    write::write_snapshot(&db, src.path()).await?;
    let files = tree(src.path());
    let text = |k: &str| String::from_utf8_lossy(&files[k]).into_owned();
    assert!(text("issues/ABC-1.md").contains("points: 3.5"));
    assert!(
        !text("issues/ABC-2.md").contains("points"),
        "no points, no line"
    );
    let rebuilt = rebuild::rebuild(src.path(), &dst.path().join("rebuilt.db")).await?;
    let points = |n| issues::get(&rebuilt, IssueId(n));
    assert_eq!(points(1).await?.points, Some(3.5));
    assert_eq!(points(2).await?.points, None);
    Ok(())
}

#[tokio::test]
async fn removed_issue_file_deleted() -> R {
    let db = seeded().await?;
    let dir = tempfile::tempdir()?;
    write::write_snapshot(&db, dir.path()).await?;
    assert!(dir.path().join("issues/ABC-2.md").exists());
    sqlx::query("DELETE FROM issues WHERE number = 2")
        .execute(&db.pool)
        .await?;
    let stats = write::write_snapshot(&db, dir.path()).await?;
    assert!(!dir.path().join("issues/ABC-2.md").exists());
    assert_eq!(stats.files_removed, 1);
    Ok(())
}

#[tokio::test]
async fn no_changes_no_commit() -> R {
    let db = seeded().await?;
    let root = tempfile::tempdir()?;
    let repo = temp_repo(root.path());
    write::write_snapshot(&db, &repo).await?;
    assert!(git::commit_and_push(&repo, "export", None).await?);
    write::write_snapshot(&db, &repo).await?;
    assert!(!git::commit_and_push(&repo, "export", None).await?);
    assert_eq!(commit_count(&repo), 1);
    Ok(())
}

#[tokio::test]
async fn secrets_never_exported() -> R {
    let db = seeded().await?;
    settings::set(&db, "gcal_token", &"TOPSECRET-1").await?;
    settings::set(&db, "gcal_client_secret", &"TOPSECRET-2").await?;
    settings::set(&db, "export_dir", &"/Users/x/TOPSECRET-3").await?;
    settings::set(
        &db,
        "export_remote",
        &"https://u:TOPSECRET-4@example.invalid/r.git",
    )
    .await?;
    let dir = tempfile::tempdir()?;
    write::write_snapshot(&db, dir.path()).await?;
    for (path, bytes) in tree(dir.path()) {
        let text = String::from_utf8(bytes)?;
        // local.yaml carries the folder and remote by design (tested apart); no credential may reach it.
        let marker = if path == "local.yaml" {
            "TOPSECRET-4"
        } else {
            "TOPSECRET"
        };
        assert!(!text.contains(marker), "{path} leaks a secret");
    }
    let local = fs::read_to_string(dir.path().join("local.yaml"))?;
    assert!(local.contains("example.invalid/r.git"));
    assert!(!local.contains("token") && !local.contains("secret"));
    let settings_file = fs::read_to_string(dir.path().join("settings.yaml"))?;
    assert!(
        settings_file.contains("gcal_enabled"),
        "ordinary settings are kept"
    );
    for key in ["token", "secret", "export_dir", "export_remote"] {
        assert!(!settings_file.contains(key), "{key} must not be exported");
    }
    Ok(())
}

#[tokio::test]
async fn title_cannot_escape_issues_dir() -> R {
    let db = Db::open_in_memory().await?;
    issues::create(
        &db,
        NewIssue {
            title: "../../escape/evil".into(),
            ..NewIssue::default()
        },
        SRC,
    )
    .await?;
    let base = tempfile::tempdir()?;
    let dir = base.path().join("export");
    write::write_snapshot(&db, &dir).await?;
    let names: Vec<String> = fs::read_dir(dir.join("issues"))?
        .map(|e| {
            e.expect("test io")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, ["ABC-1.md"]);
    let outside: Vec<String> = fs::read_dir(base.path())?
        .map(|e| {
            e.expect("test io")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        outside,
        ["export"],
        "nothing may be written outside the export dir"
    );
    Ok(())
}

#[tokio::test]
async fn push_goes_to_remote() -> R {
    let db = seeded().await?;
    let root = tempfile::tempdir()?;
    let repo = temp_repo(root.path());
    let bare = root.path().join("remote.git");
    git_in(
        root.path(),
        &["init", "-q", "--bare", bare.to_str().expect("test io")],
    );
    write::write_snapshot(&db, &repo).await?;
    assert!(git::commit_and_push(&repo, "export", bare.to_str()).await?);
    assert_eq!(commit_count(&bare), 1);
    Ok(())
}

#[tokio::test]
async fn push_failure_keeps_commit() -> R {
    let db = seeded().await?;
    let root = tempfile::tempdir()?;
    let repo = temp_repo(root.path());
    write::write_snapshot(&db, &repo).await?;
    let missing = root.path().join("no-such-remote.git");
    let result = git::commit_and_push(&repo, "export", missing.to_str()).await;
    assert!(result.is_err(), "a failed push must be an error");
    assert_eq!(commit_count(&repo), 1, "the commit stays in place");
    Ok(())
}

#[tokio::test]
async fn rebuild_refuses_existing_db() -> R {
    let db = seeded().await?;
    let dir = tempfile::tempdir()?;
    write::write_snapshot(&db, dir.path()).await?;
    let existing = dir.path().join("keep.db");
    fs::write(&existing, b"not a database")?;
    assert!(rebuild::rebuild(dir.path(), &existing).await.is_err());
    assert_eq!(
        fs::read(&existing)?,
        b"not a database",
        "an existing file must be left alone"
    );
    Ok(())
}

#[tokio::test]
async fn rebuild_existing_db_error_names_the_guard() -> R {
    let db = seeded().await?;
    let dir = tempfile::tempdir()?;
    write::write_snapshot(&db, dir.path()).await?;
    let existing = dir.path().join("keep.db");
    fs::write(&existing, b"not a database")?;
    let err = rebuild::rebuild(dir.path(), &existing)
        .await
        .err()
        .ok_or("rebuild must fail")?;
    assert!(err.to_string().contains("already exists"), "{err}");
    Ok(())
}

#[tokio::test]
async fn unsafe_project_slug_refused() -> R {
    for slug in ["a/b", "a.b", "..", ""] {
        let db = Db::open_in_memory().await?;
        if projects::create(&db, slug, "P", None, None, SRC)
            .await
            .is_err()
        {
            continue;
        }
        let dir = tempfile::tempdir()?;
        let result = write::write_snapshot(&db, dir.path()).await;
        assert!(result.is_err(), "slug {slug:?} must not become a file name");
    }
    Ok(())
}

#[tokio::test]
async fn empty_export_dir_refused() -> R {
    let db = seeded().await?;
    assert!(write::write_snapshot(&db, Path::new("")).await.is_err());
    assert!(
        git::commit_and_push(Path::new(""), "x", None)
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn export_dir_inside_other_repo_refused() -> R {
    let db = seeded().await?;
    let root = tempfile::tempdir()?;
    let repo = temp_repo(root.path());
    fs::write(repo.join("unrelated-wip.txt"), "wip")?;
    let sub = repo.join("exports");
    write::write_snapshot(&db, &sub).await?;
    let err = git::commit_and_push(&sub, "export", None)
        .await
        .err()
        .ok_or("must refuse a subfolder of another repo")?;
    assert!(
        err.to_string().contains("root of its own git repo"),
        "{err}"
    );
    let staged = git_in(&repo, &["status", "--porcelain"]);
    assert!(!staged.contains("A "), "nothing may be staged: {staged}");
    Ok(())
}

#[tokio::test]
async fn comments_heading_in_description_roundtrips() -> R {
    let db = Db::open_in_memory().await?;
    let desc = "Plan\n\n## Comments\nkeep this text and more\n\n\\## Comments\nsecond para";
    let issue = issues::create(
        &db,
        NewIssue {
            title: "T".into(),
            description: Some(desc.into()),
            ..NewIssue::default()
        },
        SRC,
    )
    .await?;
    let (src, dst) = (tempfile::tempdir()?, tempfile::tempdir()?);
    for with_comment in [false, true] {
        if with_comment {
            comments::add_comment(&db, issue.id, "c", None, SRC).await?;
        }
        write::write_snapshot(&db, src.path()).await?;
        let path = dst.path().join(format!("{with_comment}.db"));
        let rebuilt = rebuild::rebuild(src.path(), &path).await?;
        let got = issues::get(&rebuilt, issue.id).await?;
        assert_eq!(got.description, desc, "comment={with_comment}");
    }
    Ok(())
}

/// Every table the export promises to carry, as text rows without database ids (view templates keep theirs:
/// views point at them). Import log rows name their issue by number.
async fn dump(
    db: &Db,
) -> Result<BTreeMap<String, Vec<serde_json::Value>>, Box<dyn std::error::Error>> {
    let mut out = BTreeMap::new();
    for t in [
        "statuses",
        "labels",
        "projects",
        "milestones",
        "cycles",
        "views",
        "issue_templates",
        "view_templates",
        "property_defs",
        "pins",
        "import_log",
        "relation_kinds",
        "events",
        "work_blocks",
    ] {
        let cols: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM pragma_table_info(?1) WHERE (name != 'id' OR ?1 = 'view_templates')
               AND (name NOT LIKE '%!_id' ESCAPE '!' OR name = 'template_id') ORDER BY cid",
        )
        .bind(t)
        .fetch_all(&db.pool)
        .await?;
        let mut fields: Vec<String> = cols.iter().map(|c| format!("'{c}', \"{c}\"")).collect();
        if t == "import_log" || t == "work_blocks" {
            fields.push("'issue', (SELECT number FROM issues WHERE id = issue_id)".into());
        }
        let sql = format!(
            "SELECT json_object({}) AS r FROM {t} ORDER BY r",
            fields.join(", ")
        );
        let mut rows = vec![];
        for r in sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(&db.pool)
            .await?
        {
            let mut row: serde_json::Value = serde_json::from_str(&r)?;
            // JSON columns compare as values: the rebuild writes keys in sorted order.
            for c in ["body", "definition", "options"] {
                if let Some(text) = row[c].as_str() {
                    row[c] = serde_json::from_str(text)?;
                }
            }
            rows.push(row);
        }
        out.insert(t.to_string(), rows);
    }
    Ok(out)
}

#[tokio::test]
async fn rebuild_restores_every_column_the_database_holds() -> R {
    let db = seeded().await?;
    for sql in [
        "UPDATE statuses SET icon = 'icon:flag' WHERE name = 'Done'",
        "UPDATE labels SET icon = 'icon:bug', color = 'red' WHERE name = 'bug'",
        "UPDATE projects SET icon = 'emoji:🚀', color = 'teal'",
        "UPDATE cycles SET icon = 'icon:repeat', color = 'blue'",
        "INSERT INTO view_templates (id, name, definition, position, icon, color)
           VALUES (7, 'Board', '{\"filter\":null,\"sorts\":[],\"group_by\":null,\"layout\":\"Board\",\"columns\":[],\"include_archived\":false}', 0, 'icon:kanban', 'indigo')",
        "UPDATE views SET icon = 'icon:star', color = 'yellow', template_id = 7 WHERE id = (SELECT min(id) FROM views)",
        "INSERT INTO issue_templates (name, slug, icon, color, body)
           VALUES ('Bug report', 'bug-report', 'icon:bug', 'red', '{\"title_prefix\":\"Bug: \",\"fields\":{\"priority\":2},\"sub_issues\":[]}')",
        "INSERT INTO import_log (external_id, source, issue_id, imported_at)
           VALUES ('row-1', 'csv', (SELECT id FROM issues WHERE number = 1), '2026-01-02T03:04:05Z')",
        "INSERT INTO events (id, title, starts_at, ends_at, all_day, place, notes, icon, color)
           VALUES (4, 'Offsite', '2026-12-10 09:00:00', '2026-12-11 17:00:00', 0, 'Lisbon', 'Bring slides', 'icon:flag', 'teal')",
        "INSERT INTO event_issues (event_id, issue_id) VALUES (4, (SELECT id FROM issues WHERE number = 1))",
        "UPDATE import_log SET source_deleted = 1",
        "INSERT INTO work_blocks (issue_id, start_utc, end_utc)
           VALUES ((SELECT id FROM issues WHERE number = 2), '2026-01-05T09:00:00+00:00', '2026-01-05T10:30:00+00:00')",
    ] {
        sqlx::query(sql).execute(&db.pool).await?;
    }
    let (src, dst) = (tempfile::tempdir()?, tempfile::tempdir()?);
    write::write_snapshot(&db, src.path()).await?;
    let rebuilt = rebuild::rebuild(src.path(), &dst.path().join("rebuilt.db")).await?;
    let (want, got) = (dump(&db).await?, dump(&rebuilt).await?);
    for (table, rows) in &want {
        assert_eq!(&got[table], rows, "table {table}");
    }
    let linked = sqlx::query_scalar::<_, i64>(
        "SELECT i.number FROM event_issues e JOIN issues i ON i.id = e.issue_id WHERE e.event_id = 4",
    )
    .fetch_all(&rebuilt.pool)
    .await?;
    assert_eq!(linked, vec![1]);
    Ok(())
}

#[tokio::test]
async fn a_hand_edited_slug_or_prefix_still_cannot_become_a_file_name() -> R {
    for (sql, what) in [
        ("UPDATE projects SET slug = '../up'", "slug"),
        (
            "UPDATE settings SET value = '\"../x\"' WHERE key = 'issue_prefix'",
            "prefix",
        ),
    ] {
        let db = seeded().await?;
        sqlx::query(sql).execute(&db.pool).await?;
        let dir = tempfile::tempdir()?;
        let result = write::write_snapshot(&db, dir.path()).await;
        assert!(result.is_err(), "a hand-edited {what} was written");
        assert_eq!(fs::read_dir(dir.path())?.count(), 0, "nothing is written");
    }
    Ok(())
}

#[tokio::test]
async fn custom_relation_kinds_and_their_relations_survive_export_and_rebuild() -> R {
    let db = seeded().await?;
    let k = relation_kinds::create(&db, "Mentors", "mentors", "mentored by", false, SRC).await?;
    for (from, to, kind) in [(1, 2, k.id.0.as_str()), (2, 1, "follows")] {
        relations::add(
            &db,
            Relation {
                from: IssueId(from),
                to: IssueId(to),
                kind: RelationKind::new(kind),
            },
            SRC,
        )
        .await?;
    }
    let (src, dst) = (tempfile::tempdir()?, tempfile::tempdir()?);
    write::write_snapshot(&db, src.path()).await?;
    let rebuilt = rebuild::rebuild(src.path(), &dst.path().join("rebuilt.db")).await?;
    assert_eq!(
        relation_kinds::list(&rebuilt).await?,
        relation_kinds::list(&db).await?
    );
    for id in [1, 2] {
        assert_eq!(
            relations::for_issue(&rebuilt, IssueId(id)).await?,
            relations::for_issue(&db, IssueId(id)).await?
        );
    }
    Ok(())
}

/// A source database exported, committed and pushed to a local bare repo, then cloned the way the app does.
async fn pushed_and_cloned(db: &Db, root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo = temp_repo(root);
    let bare = root.join("remote.git");
    git_in(
        root,
        &["init", "-q", "--bare", bare.to_str().expect("test io")],
    );
    write::write_snapshot(db, &repo).await?;
    git::commit_and_push(&repo, "export", bare.to_str()).await?;
    let clone = root.join("clone");
    git_in(
        root,
        &[
            "clone",
            "-q",
            bare.to_str().expect("test io"),
            clone.to_str().expect("test io"),
        ],
    );
    Ok(clone)
}

#[tokio::test]
async fn restore_replaces_a_used_workspace_keeps_a_backup_and_continues_the_history() -> R {
    let src = seeded().await?;
    settings::set(
        &src,
        "export_remote",
        &"https://me:hunter2@example.invalid/x.git",
    )
    .await?;
    settings::set(&src, "backup_dir", &"/somewhere/else").await?;
    let root = tempfile::tempdir()?;
    let clone = pushed_and_cloned(&src, root.path()).await?;
    assert!(!fs::read_to_string(clone.join("local.yaml"))?.contains("hunter2"));

    // A workspace with other data and a change history of its own.
    // A file database, as in the app (VACUUM INTO writes nothing for a memory one).
    let used = Db::open(&root.path().join("used.db")).await?;
    let (docs, ship) = seed_issues(&used).await?;
    seed_rest(&used, docs, ship).await?;
    labels::create(&used, "stale-label", "red", None, SRC).await?;
    settings::set(&used, "estimation", &"points").await?;
    let backups = root.path().join("backups");
    let out = taskapp_export::restore::restore_from_dir(
        &used,
        &clone,
        Some("git@example.invalid:me/x.git"),
        &backups,
        false,
    )
    .await?;

    assert_eq!(dump(&used).await?, dump(&src).await?);
    assert!(
        out.backup.exists(),
        "the old database was backed up first: {:?}",
        out.backup
    );
    let history: i64 =
        sqlx::query_scalar("SELECT count(*) FROM changes WHERE entity != 'row:settings'")
            .fetch_one(&used.pool)
            .await?;
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT entity, entity_id, field FROM changes")
            .fetch_all(&used.pool)
            .await?;
    assert_eq!(
        history, 0,
        "old row ids would point at the wrong rows: {rows:?}"
    );
    let get = |k: &'static str| settings::get::<String>(&used, k);
    assert_eq!(get("export_dir").await?.as_deref(), clone.to_str());
    assert_eq!(
        get("export_remote").await?.as_deref(),
        Some("git@example.invalid:me/x.git")
    );
    assert_eq!(
        get("backup_dir").await?,
        None,
        "local settings are offered, not forced"
    );
    Ok(())
}

#[tokio::test]
async fn the_next_export_after_a_restore_builds_on_the_cloned_history() -> R {
    let src = seeded().await?;
    let root = tempfile::tempdir()?;
    let clone = pushed_and_cloned(&src, root.path()).await?;
    let fresh = Db::open_in_memory().await?;
    taskapp_export::restore::restore_from_dir(&fresh, &clone, None, &root.path().join("b"), false)
        .await?;

    // The next export differs only in local.yaml and setup_done (this machine's settings) and builds on the clone's history.
    write::write_snapshot(&fresh, &clone).await?;
    assert_eq!(
        git_in(&clone, &["status", "--porcelain", "-uall"])
            .lines()
            .count(),
        2
    );
    git_in(&clone, &["config", "user.name", "t"]);
    git_in(&clone, &["config", "user.email", "t@example.invalid"]);
    assert!(git::commit_and_push(&clone, "export", None).await?);
    assert_eq!(commit_count(&clone), 2);
    Ok(())
}

#[tokio::test]
async fn restore_offers_the_local_settings_when_asked() -> R {
    let src = seeded().await?;
    settings::set(&src, "backup_dir", &"~/Backups").await?;
    let root = tempfile::tempdir()?;
    let clone = pushed_and_cloned(&src, root.path()).await?;
    let fresh = Db::open_in_memory().await?;
    taskapp_export::restore::restore_from_dir(&fresh, &clone, None, &root.path().join("b"), true)
        .await?;
    assert_eq!(
        settings::get::<String>(&fresh, "backup_dir")
            .await?
            .as_deref(),
        Some("~/Backups")
    );
    assert_eq!(
        settings::get::<bool>(&fresh, "setup_done").await?,
        Some(true)
    );
    Ok(())
}

#[tokio::test]
async fn a_broken_export_leaves_the_workspace_as_it_was() -> R {
    let src = seeded().await?;
    let root = tempfile::tempdir()?;
    let clone = pushed_and_cloned(&src, root.path()).await?;
    fs::write(clone.join("statuses.yaml"), "not: [a list")?;
    let used = seeded().await?;
    let before = dump(&used).await?;
    let err = taskapp_export::restore::restore_from_dir(
        &used,
        &clone,
        None,
        &root.path().join("b"),
        false,
    )
    .await;
    assert!(err.is_err());
    assert_eq!(dump(&used).await?, before);
    let issues: i64 = sqlx::query_scalar("SELECT count(*) FROM issues")
        .fetch_one(&used.pool)
        .await?;
    assert!(issues > 0);
    Ok(())
}

#[tokio::test]
async fn a_folder_with_a_symlink_is_refused() -> R {
    let src = seeded().await?;
    let root = tempfile::tempdir()?;
    let clone = pushed_and_cloned(&src, root.path()).await?;
    fs::remove_file(clone.join("pins.yaml"))?;
    std::os::unix::fs::symlink("/etc/hosts", clone.join("pins.yaml"))?;
    let used = Db::open_in_memory().await?;
    let err = taskapp_export::restore::restore_from_dir(
        &used,
        &clone,
        None,
        &root.path().join("b"),
        false,
    )
    .await;
    assert!(err.is_err());
    Ok(())
}
