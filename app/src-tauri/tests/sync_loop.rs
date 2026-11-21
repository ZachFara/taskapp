use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
    time::{Duration, Instant},
};

use taskapp_app::{
    deeplink::parse_key,
    sync_loop::{Real, Step, Steps, Syncer, Timing, poll_db, run, watch_reminders},
};
use taskapp_core::{Db, Error, Result, SyncState, settings};
use tokio::{sync::mpsc, time::sleep};

type R = std::result::Result<(), Box<dyn std::error::Error>>;

/// Records every step, and how many ran at once.
#[derive(Default)]
struct Fake {
    log: Mutex<Vec<Step>>,
    active: AtomicUsize,
    most: AtomicUsize,
    fail: Option<Step>,
}

/// `Steps` for a shared `Fake`, so the test keeps a handle to read the log.
struct Handle(Arc<Fake>);

impl Steps for Handle {
    async fn run(&self, step: Step) -> Result<()> {
        let this = &self.0;
        let now = this.active.fetch_add(1, SeqCst) + 1;
        this.most.fetch_max(now, SeqCst);
        sleep(Duration::from_millis(15)).await;
        this.log
            .lock()
            .map_err(|_| Error::Invalid("poisoned".into()))?
            .push(step);
        this.active.fetch_sub(1, SeqCst);
        if this.fail == Some(step) {
            return Err(Error::Invalid("boom".into()));
        }
        Ok(())
    }
}

fn count(f: &Fake, step: Step) -> usize {
    f.log
        .lock()
        .map_or(0, |l| l.iter().filter(|s| **s == step).count())
}

#[tokio::test]
async fn syncs_never_overlap() -> R {
    let fake = Arc::new(Fake::default());
    let syncer = Arc::new(Syncer::new(
        Handle(fake.clone()),
        Db::open_in_memory().await?,
    ));
    let a = tokio::spawn({
        let s = syncer.clone();
        async move { s.sync_all().await }
    });
    let b = tokio::spawn({
        let s = syncer.clone();
        async move { s.sync_all().await }
    });
    syncer.step(Step::Gcal).await;
    a.await??;
    b.await??;
    assert_eq!(fake.most.load(SeqCst), 1);
    assert_eq!(fake.log.lock().map_err(|_| "poisoned")?.len(), 9);
    Ok(())
}

#[tokio::test]
async fn sync_now_reports_each_state() -> R {
    // A failing step is recorded and the steps after it still run.
    let fake = Arc::new(Fake {
        fail: Some(Step::Reminders),
        ..Fake::default()
    });
    let db = Db::open_in_memory().await?;
    let status = Syncer::new(Handle(fake.clone()), db).sync_all().await?;
    assert!(
        matches!(&status.reminders, SyncState::Error { message, .. } if message.contains("boom"))
    );
    assert_eq!(status.gcal, SyncState::Off);
    assert_eq!(count(&fake, Step::Export), 1);

    // The real steps on an empty database: nothing is configured, so nothing runs and nothing errors.
    let dir = tempfile::tempdir()?;
    let db = Db::open(&dir.path().join("t.db")).await?;
    let real = Real {
        db: db.clone(),
        helper: PathBuf::from("/nonexistent/helper"),
        tz: "UTC".into(),
        backup_dir: dir.path().join("backups"),
    };
    let status = Syncer::new(real, db.clone()).sync_all().await?;
    assert_eq!(
        (status.reminders, status.gcal, status.export),
        (SyncState::Off, SyncState::Off, SyncState::Off)
    );
    assert!(dir.path().join("backups/nightly").exists());

    // Export enabled without a folder is an error state, not a silent skip.
    settings::set(&db, "export_enabled", &true).await?;
    let real = Real {
        db: db.clone(),
        helper: PathBuf::new(),
        tz: "UTC".into(),
        backup_dir: dir.path().join("backups"),
    };
    let status = Syncer::new(real, db).sync_all().await?;
    assert!(matches!(status.export, SyncState::Error { .. }));
    Ok(())
}

fn fast() -> Timing {
    let hour = Duration::from_secs(3600);
    Timing {
        reminders: hour,
        calendar: hour,
        calendar_wait: Duration::from_millis(150),
        export: hour,
        backup: hour,
        snapshots: hour,
    }
}

#[tokio::test]
async fn calendar_debounce_coalesces_changes() -> R {
    let fake = Arc::new(Fake::default());
    let syncer = Arc::new(Syncer::new(
        Handle(fake.clone()),
        Db::open_in_memory().await?,
    ));
    let (cal_tx, cal_rx) = mpsc::unbounded_channel();
    let (_rem_tx, rem_rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(run(syncer, fast(), PathBuf::new(), cal_rx, rem_rx));
    sleep(Duration::from_millis(200)).await; // the start-up sync
    assert_eq!(count(&fake, Step::Gcal), 1);
    let last = Instant::now();
    for _ in 0..4 {
        cal_tx.send(())?;
        sleep(Duration::from_millis(50)).await;
    }
    sleep(Duration::from_millis(100)).await;
    assert_eq!(count(&fake, Step::Gcal), 1, "still waiting for quiet");
    sleep(Duration::from_millis(300)).await;
    assert_eq!(count(&fake, Step::Gcal), 2, "one run for four changes");
    assert!(last.elapsed() >= Duration::from_millis(350));
    task.abort();
    Ok(())
}

#[tokio::test]
async fn new_snapshot_file_triggers_export() -> R {
    let fake = Arc::new(Fake::default());
    let dir = tempfile::tempdir()?;
    let syncer = Arc::new(Syncer::new(
        Handle(fake.clone()),
        Db::open_in_memory().await?,
    ));
    let (_cal_tx, cal_rx) = mpsc::unbounded_channel();
    let (_rem_tx, rem_rx) = mpsc::unbounded_channel();
    let timing = Timing {
        snapshots: Duration::from_millis(50),
        ..fast()
    };
    let task = tokio::spawn(run(syncer, timing, dir.path().into(), cal_rx, rem_rx));
    sleep(Duration::from_millis(200)).await;
    assert_eq!(count(&fake, Step::Export), 1);
    std::fs::create_dir_all(dir.path().join("snapshots"))?;
    std::fs::write(dir.path().join("snapshots/x.db"), b"")?;
    sleep(Duration::from_millis(250)).await;
    assert_eq!(count(&fake, Step::Export), 2);
    task.abort();
    Ok(())
}

#[tokio::test]
async fn app_refreshes_on_external_change() -> R {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("t.db");
    let app = Db::open(&path).await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let poll = tokio::spawn(poll_db(app, Duration::from_secs(1), move |calendar| {
        tx.send(calendar).ok();
    }));
    sleep(Duration::from_millis(200)).await;
    // A second handle, as the MCP server or CLI would be.
    let other = Db::open(&path).await?;
    settings::set(&other, "issue_prefix", &"AB").await?;
    let started = Instant::now();
    let calendar = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await?;
    assert_eq!(
        calendar,
        Some(false),
        "a settings write is not a calendar change"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    sqlx::query("INSERT INTO statuses (name, category, position, color) VALUES ('x', 'backlog', 99, '#fff')")
        .execute(&other.pool)
        .await?;
    let calendar = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await?;
    assert_eq!(calendar, Some(true));
    poll.abort();
    Ok(())
}

#[tokio::test]
async fn reminders_watch_forwards_change_lines() -> R {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let helper = dir.path().join("helper");
    std::fs::write(
        &helper,
        "#!/bin/sh\necho '{\"changed\":true}'\necho '{\"changed\":true}'\n",
    )?;
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755))?;
    let db = Db::open(&dir.path().join("t.db")).await?;
    settings::set(&db, "reminders_list_id", &"list-1").await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(watch_reminders(db, helper, tx));
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(5), rx.recv()).await?;
    }
    task.abort();
    Ok(())
}

#[test]
fn deeplink_parses_key() {
    assert_eq!(parse_key("taskapp://issue/ZF-12").as_deref(), Some("ZF-12"));
    assert_eq!(
        parse_key("taskapp://issue/ZF-12/").as_deref(),
        Some("ZF-12")
    );
    for bad in [
        "taskapp://issue/",
        "taskapp://issue/zf-1",
        "taskapp://issue/ZF-",
        "taskapp://issue/ZF-1x",
        "taskapp://issue/ZF-1/../x",
        "taskapp://issue/ZF-1?x=1",
        "taskapp://issue/-1",
        "taskapp://issue/ZF-99999999999",
        "taskapp://project/ZF-1",
        "https://issue/ZF-1",
    ] {
        assert_eq!(parse_key(bad), None, "{bad}");
    }
}

#[tokio::test]
async fn backup_failure_is_recorded_and_clears_on_success() -> R {
    let dir = tempfile::tempdir()?;
    let db = Db::open(&dir.path().join("t.db")).await?;
    // A file where the backup folder should be makes the nightly write fail.
    std::fs::write(dir.path().join("blocked"), b"")?;
    let real = Real {
        db: db.clone(),
        helper: PathBuf::from("/nonexistent/helper"),
        tz: "UTC".into(),
        backup_dir: dir.path().join("blocked/backups"),
    };
    let status = Syncer::new(real, db.clone()).sync_all().await?;
    assert!(
        matches!(&status.backup, SyncState::Error { message, .. } if !message.is_empty()),
        "{:?}",
        status.backup
    );

    let real = Real {
        db: db.clone(),
        helper: PathBuf::from("/nonexistent/helper"),
        tz: "UTC".into(),
        backup_dir: dir.path().join("backups"),
    };
    let status = Syncer::new(real, db).sync_all().await?;
    assert!(
        matches!(status.backup, SyncState::Ok { .. }),
        "{:?}",
        status.backup
    );
    Ok(())
}

#[tokio::test]
async fn sync_status_saved_before_backup_existed_still_reads() -> R {
    let db = Db::open_in_memory().await?;
    settings::set(
        &db,
        "sync_status",
        &serde_json::json!({"reminders": "Off", "gcal": "Off", "export": "Off"}),
    )
    .await?;
    let status = Syncer::new(Handle(Arc::new(Fake::default())), db)
        .sync_all()
        .await?;
    assert_eq!(status.backup, SyncState::Off);
    Ok(())
}
