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
    properties, relations, settings, views,
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
    let app = projects::create(db, "app", "App", SRC).await?;
    let v1 = projects::create_milestone(db, app.id, "v1", Some(day(2026, 3, 1)), SRC).await?;
    let sprint = cycles::create(
        db,
        Some("Sprint".into()),
        day(2026, 1, 1),
        day(2026, 1, 14),
        SRC,
    )
    .await?;
    let bug = labels::create(db, "bug", "#ff0000", SRC).await?;
    labels::create(db, "backend", "#00ff00", SRC).await?;
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
                tz: "America/New_York".into(),
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
            kind: RelationKind::Blocks,
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
    properties::set_value(db, docs, "owner", Some(json!("zach")), SRC).await?;
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
        },
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
async fn removed_issue_file_deleted() -> R {
    let db = seeded().await?;
    let dir = tempfile::tempdir()?;
    write::write_snapshot(&db, dir.path()).await?;
    assert!(dir.path().join("issues/ZF-2.md").exists());
    sqlx::query("DELETE FROM issues WHERE number = 2")
        .execute(&db.pool)
        .await?;
    let stats = write::write_snapshot(&db, dir.path()).await?;
    assert!(!dir.path().join("issues/ZF-2.md").exists());
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
        assert!(!text.contains("TOPSECRET"), "{path} leaks a secret");
    }
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
    assert_eq!(names, ["ZF-1.md"]);
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
