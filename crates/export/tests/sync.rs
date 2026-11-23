//! Two machines and a local bare repository standing in for the remote.

use std::path::{Path, PathBuf};

use taskapp_core::{
    ChangeSource, Db, IssueId, IssuePatch, NewIssue, comments, issues, labels, undo,
};
use taskapp_export::{
    git::{self, RemoteState},
    rebuild,
    sync::{self, REMOTE_AHEAD},
};

type R = Result<(), Box<dyn std::error::Error>>;

const SRC: ChangeSource = ChangeSource::Ui;

fn git_in(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

async fn issue(db: &Db, title: &str) -> Result<IssueId, Box<dyn std::error::Error>> {
    let new = NewIssue {
        title: title.into(),
        ..NewIssue::default()
    };
    Ok(issues::create(db, new, SRC).await?.id)
}

async fn retitle(db: &Db, id: IssueId, title: &str) -> R {
    let patch = IssuePatch {
        title: Some(title.into()),
        ..IssuePatch::default()
    };
    issues::update(db, id, patch, SRC).await?;
    Ok(())
}

async fn title(db: &Db, number: i64) -> Result<String, Box<dyn std::error::Error>> {
    Ok(issues::get_by_key(db, &format!("ABC-{number}"))
        .await?
        .title)
}

/// Machine A has issues 1 and 2 pushed; machine B is a clone rebuilt into its own database.
struct Pair {
    root: tempfile::TempDir,
    bare: PathBuf,
    dir_a: PathBuf,
    dir_b: PathBuf,
    a: Db,
    b: Db,
}

impl Pair {
    fn remote(&self) -> &str {
        self.bare.to_str().expect("utf8")
    }

    async fn new() -> Result<Pair, Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let (bare, dir_a, dir_b) = (
            root.path().join("r.git"),
            root.path().join("a"),
            root.path().join("b"),
        );
        git_in(
            root.path(),
            &[
                "init",
                "-q",
                "--bare",
                "-b",
                "main",
                bare.to_str().expect("utf8"),
            ],
        );
        std::fs::create_dir(&dir_a)?;
        git_in(&dir_a, &["init", "-q", "-b", "main"]);
        let a = Db::open_in_memory().await?;
        labels::create(&a, "bug", "red", None, SRC).await?;
        let one = issue(&a, "One").await?;
        issue(&a, "Two").await?;
        comments::add_comment(&a, one, "hello", None, SRC).await?;
        let remote = bare.to_str().expect("utf8");
        sync::export(&a, &dir_a, Some(remote)).await?;
        git_in(
            root.path(),
            &["clone", "-q", remote, dir_b.to_str().expect("utf8")],
        );
        let b = rebuild::rebuild(&dir_b, &root.path().join("b.db")).await?;
        Ok(Pair {
            root,
            bare,
            dir_a,
            dir_b,
            a,
            b,
        })
    }
}

#[tokio::test]
async fn export_refuses_to_overwrite_remote_changes_then_pull_applies_them_as_one_undoable_step()
-> R {
    let p = Pair::new().await?;
    retitle(&p.a, IssueId(1), "One v2").await?;
    let three = issue(&p.a, "Three").await?;
    comments::add_comment(&p.a, IssueId(2), "late note", None, SRC).await?;
    sync::export(&p.a, &p.dir_a, Some(p.remote())).await?;

    // B sees the new commits, and its own export is refused instead of overwriting them.
    assert_eq!(
        git::remote_state(&p.dir_b, p.remote()).await?,
        RemoteState::Behind
    );
    let refused = sync::export(&p.b, &p.dir_b, Some(p.remote())).await;
    assert!(refused.is_err_and(|e| e.to_string().contains(REMOTE_AHEAD)));

    let out = sync::pull_and_apply(&p.b, &p.dir_b, p.remote()).await?;
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(out.created, 2, "issue 1 changed, issue 3 created");
    assert_eq!(title(&p.b, 1).await?, "One v2");
    assert_eq!(title(&p.b, 3).await?, "Three");
    assert_eq!(three, IssueId(3));
    assert_eq!(comments::list(&p.b, IssueId(2)).await?.len(), 1);
    assert_eq!(
        git::remote_state(&p.dir_b, p.remote()).await?,
        RemoteState::Current
    );

    // B's export after applying is identical to A's, so there is nothing for A to pull back.
    sync::export(&p.b, &p.dir_b, Some(p.remote())).await?;
    assert_eq!(
        git::remote_state(&p.dir_a, p.remote()).await?,
        RemoteState::Current
    );

    let status = undo::undo_status(&p.b).await?;
    assert_eq!(status.undo.as_deref(), Some("Sync from remote"));
    undo::undo(&p.b).await?;
    assert_eq!(title(&p.b, 1).await?, "One");
    assert!(
        issues::get_by_key(&p.b, "ABC-3")
            .await?
            .archived_at
            .is_some()
    );
    assert_eq!(comments::list(&p.b, IssueId(2)).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn the_newer_edit_wins_per_issue_and_the_conflict_is_reported() -> R {
    let p = Pair::new().await?;
    retitle(&p.a, IssueId(1), "A one").await?;
    retitle(&p.a, IssueId(2), "A two").await?;
    sync::export(&p.a, &p.dir_a, Some(p.remote())).await?;
    retitle(&p.b, IssueId(2), "B two").await?; // later than A's edit of issue 2

    let out = sync::pull_and_apply(&p.b, &p.dir_b, p.remote()).await?;
    assert_eq!(title(&p.b, 1).await?, "A one");
    assert_eq!(title(&p.b, 2).await?, "B two");
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].starts_with("ABC-2"));
    Ok(())
}

#[tokio::test]
async fn diverged_histories_are_refused_not_merged() -> R {
    let p = Pair::new().await?;
    retitle(&p.a, IssueId(1), "A one").await?;
    sync::export(&p.a, &p.dir_a, Some(p.remote())).await?;
    retitle(&p.b, IssueId(2), "B two").await?;
    // B commits locally but cannot push: the remote has moved.
    taskapp_export::write::write_snapshot(&p.b, &p.dir_b).await?;
    assert!(
        git::commit_and_push(&p.dir_b, "export", Some(p.remote()))
            .await
            .is_err()
    );
    assert_eq!(
        git::remote_state(&p.dir_b, p.remote()).await?,
        RemoteState::Diverged
    );
    assert!(
        sync::pull_and_apply(&p.b, &p.dir_b, p.remote())
            .await
            .is_err()
    );
    assert_eq!(title(&p.b, 1).await?, "One");
    Ok(())
}

#[tokio::test]
async fn an_empty_remote_is_current() -> R {
    let root = tempfile::tempdir()?;
    let bare = root.path().join("r.git");
    git_in(
        root.path(),
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            bare.to_str().expect("utf8"),
        ],
    );
    let dir = root.path().join("a");
    std::fs::create_dir(&dir)?;
    git_in(&dir, &["init", "-q", "-b", "main"]);
    assert_eq!(
        git::remote_state(&dir, bare.to_str().expect("utf8")).await?,
        RemoteState::Current
    );
    Ok(())
}

/// A second clone pushes `change` as a commit on the remote; returns the temp dir that must stay untouched.
fn push_link_commit(
    p: &Pair,
    replace_issues_dir: bool,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("keep.md"), "outside")?;
    let evil = p.root.path().join("evil");
    git_in(
        p.root.path(),
        &["clone", "-q", p.remote(), evil.to_str().expect("utf8")],
    );
    let target = outside.path();
    if replace_issues_dir {
        git_in(&evil, &["rm", "-rq", "issues"]);
        std::os::unix::fs::symlink(target, evil.join("issues"))?;
    } else {
        std::os::unix::fs::symlink(target, evil.join("issues").join("ABC-9.md"))?;
    }
    git_in(&evil, &["add", "-A"]);
    git_in(&evil, &["commit", "-q", "-m", "link"]);
    git_in(&evil, &["push", "-q", "origin", "HEAD"]);
    Ok(outside)
}

#[tokio::test]
async fn a_remote_with_a_symlink_is_refused_and_nothing_is_applied_or_touched() -> R {
    for replace_dir in [true, false] {
        let p = Pair::new().await?;
        let outside = push_link_commit(&p, replace_dir)?;
        assert_eq!(
            git::remote_state(&p.dir_b, p.remote()).await?,
            RemoteState::Behind
        );
        let err = sync::pull_and_apply(&p.b, &p.dir_b, p.remote())
            .await
            .expect_err("refused");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert_eq!(title(&p.b, 1).await?, "One");
        assert!(!p.dir_b.join("issues").is_symlink());
        assert_eq!(std::fs::read_dir(outside.path())?.count(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn export_and_rebuild_never_follow_links_in_the_folder() -> R {
    let p = Pair::new().await?;
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("ABC-1.md"), "outside")?;
    // The managed directory is a link: no write, no delete through it.
    std::fs::remove_dir_all(p.dir_b.join("issues"))?;
    std::os::unix::fs::symlink(outside.path(), p.dir_b.join("issues"))?;
    assert!(
        taskapp_export::write::write_snapshot(&p.b, &p.dir_b)
            .await
            .is_err()
    );
    assert!(
        rebuild::rebuild(&p.dir_b, &p.dir_b.join("x.db"))
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(outside.path().join("ABC-1.md"))?,
        "outside"
    );
    // A file at a managed path is a link: refused, target untouched.
    std::fs::remove_file(p.dir_b.join("issues"))?;
    std::fs::create_dir(p.dir_b.join("issues"))?;
    std::os::unix::fs::symlink(
        outside.path().join("ABC-1.md"),
        p.dir_b.join("issues").join("ABC-1.md"),
    )?;
    assert!(
        taskapp_export::write::write_snapshot(&p.b, &p.dir_b)
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(outside.path().join("ABC-1.md"))?,
        "outside"
    );
    Ok(())
}
