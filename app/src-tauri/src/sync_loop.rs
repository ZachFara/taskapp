//! Background sync loop, started by the app: one task runs every sync, so syncs never overlap.

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use sqlx::SqliteConnection;
use taskapp_core::{Db, Error, Result, SyncState, SyncStatus, backup, settings};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    time::{Instant, Interval, interval_at, sleep, sleep_until},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Backup,
    Reminders,
    Gcal,
    Export,
}

/// The order `sync_now` and the first run at start use.
const ORDER: [Step; 4] = [Step::Backup, Step::Reminders, Step::Gcal, Step::Export];

/// What each step does. The real one talks to Reminders, Google and git; tests use a fake.
pub trait Steps: Send + Sync + 'static {
    fn run(&self, step: Step) -> impl Future<Output = Result<()>> + Send;
}

pub struct Syncer<S> {
    steps: S,
    db: Db,
    lock: tokio::sync::Mutex<()>,
}

impl<S: Steps> Syncer<S> {
    pub fn new(steps: S, db: Db) -> Self {
        Syncer {
            steps,
            db,
            lock: tokio::sync::Mutex::new(()),
        }
    }

    /// A failing step is recorded in the status and does not stop the caller.
    async fn run_step(&self, step: Step) {
        if let Err(e) = self.steps.run(step).await {
            eprintln!("sync {step:?} failed: {e}");
            let state = SyncState::Error {
                at: Utc::now(),
                message: e.to_string(),
            };
            if let Err(e) = set_state(&self.db, step, state).await {
                eprintln!("sync {step:?}: cannot record the error: {e}");
            }
        }
    }

    /// Runs one step; waits for any other sync to finish first.
    pub async fn step(&self, step: Step) {
        let _turn = self.lock.lock().await;
        self.run_step(step).await;
    }

    /// Runs every step in order, then returns the status.
    ///
    /// # Errors
    /// Fails only when the status cannot be read.
    pub async fn sync_all(&self) -> Result<SyncStatus> {
        let _turn = self.lock.lock().await;
        for step in ORDER {
            self.run_step(step).await;
        }
        status(&self.db).await
    }
}

async fn status(db: &Db) -> Result<SyncStatus> {
    Ok(settings::get(db, "sync_status")
        .await?
        .unwrap_or(SyncStatus {
            reminders: SyncState::Off,
            gcal: SyncState::Off,
            export: SyncState::Off,
        }))
}

/// Backups have no status field; their failures only reach the log.
async fn set_state(db: &Db, step: Step, state: SyncState) -> Result<()> {
    let mut all = status(db).await?;
    match step {
        Step::Reminders => all.reminders = state,
        Step::Gcal => all.gcal = state,
        Step::Export => all.export = state,
        Step::Backup => return Ok(()),
    }
    settings::set(db, "sync_status", &all).await
}

pub struct Real {
    pub db: Db,
    pub helper: PathBuf,
    pub tz: String,
    pub backup_dir: PathBuf,
}

impl Steps for Real {
    async fn run(&self, step: Step) -> Result<()> {
        match step {
            Step::Backup => {
                backup::nightly(&self.db, &self.backup_dir, Utc::now()).await?;
                backup::prune(&self.backup_dir)?;
            }
            Step::Reminders => {
                let tz: chrono_tz::Tz = self
                    .tz
                    .parse()
                    .map_err(|_| Error::Invalid(format!("unknown time zone {}", self.tz)))?;
                let today = Utc::now().with_timezone(&tz).date_naive();
                taskapp_reminders::import::import(&self.db, &self.helper, today, &self.tz).await?;
            }
            Step::Gcal => {
                taskapp_gcal::reconcile::reconcile(&self.db).await?;
            }
            Step::Export => export(&self.db).await?,
        }
        Ok(())
    }
}

async fn export(db: &Db) -> Result<()> {
    if !settings::get::<bool>(db, "export_enabled")
        .await?
        .unwrap_or(false)
    {
        return Ok(());
    }
    let dir: String = settings::get(db, "export_dir")
        .await?
        .ok_or_else(|| Error::Invalid("choose an export folder in Settings first".into()))?;
    let remote: Option<String> = settings::get(db, "export_remote").await?;
    taskapp_export::write::write_snapshot(db, Path::new(&dir)).await?;
    taskapp_export::git::commit_and_push(Path::new(&dir), "export", remote.as_deref()).await?;
    set_state(db, Step::Export, SyncState::Ok { at: Utc::now() }).await
}

/// The bundled helper, or in a debug build the one built in the repo when the bundle has none.
#[must_use]
pub fn resolve_helper(bundled: PathBuf) -> PathBuf {
    let built = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../helpers/reminders-helper/.build/release/RemindersHelper");
    if cfg!(debug_assertions) && !bundled.exists() {
        built
    } else {
        bundled
    }
}

/// Counts as a calendar change when it differs between two reads. Sync's own writes (event map, status)
/// touch none of these, so a reconcile cannot trigger itself.
/// shortcut: any issue, status or work block edit counts, not only fields that reach the calendar.
const CALENDAR_FINGERPRINT: &str = "SELECT \
    (SELECT coalesce(max(id), 0) FROM changes) || '/' || \
    (SELECT count(*) || coalesce(max(updated_at), '') FROM issues) || '/' || \
    (SELECT count(*) || coalesce(max(start_utc), '') || coalesce(max(end_utc), '') FROM work_blocks) || '/' || \
    (SELECT count(*) || coalesce(max(id), 0) FROM statuses)";

async fn read(conn: &mut SqliteConnection) -> sqlx::Result<(i64, String)> {
    let version = sqlx::query_scalar("PRAGMA data_version")
        .fetch_one(&mut *conn)
        .await?;
    let print = sqlx::query_scalar(CALENDAR_FINGERPRINT)
        .fetch_one(&mut *conn)
        .await?;
    Ok((version, print))
}

/// Polls `data_version` on one dedicated connection (the value is per connection) and calls
/// `on_change(calendar_relevant)` whenever another connection committed. Never returns.
///
/// # Errors
/// Fails when no connection can be taken; a failed poll is retried on the next tick.
pub async fn poll_db(db: Db, every: Duration, on_change: impl Fn(bool) + Send) -> Result<()> {
    let mut conn = db.pool.acquire().await?;
    let (mut version, mut print) = read(&mut conn).await?;
    let mut tick = tokio::time::interval(every);
    loop {
        tick.tick().await;
        match read(&mut conn).await {
            Ok((v, _)) if v == version => {}
            Ok((v, p)) => {
                version = v;
                on_change(p != print);
                print = p;
            }
            Err(e) => eprintln!("data_version poll failed: {e}"),
        }
    }
}

/// Runs the helper's `watch` and sends one message per change line. Restarts it when it exits.
pub async fn watch_reminders(db: Db, helper: PathBuf, tx: UnboundedSender<()>) {
    loop {
        let list: String = settings::get(&db, "reminders_list_id")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if !list.is_empty()
            && let Err(e) = watch_once(&helper, &list, &tx).await
        {
            eprintln!("reminders watch failed: {e}");
        }
        sleep(Duration::from_secs(60)).await;
    }
}

async fn watch_once(helper: &Path, list: &str, tx: &UnboundedSender<()>) -> std::io::Result<()> {
    // The helper runs until its stdin closes, so the pipe stays open as long as `child` lives.
    let mut child = Command::new(helper)
        .args(["watch", "--list", list])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let out = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no stdout"))?;
    let mut lines = BufReader::new(out).lines();
    while lines.next_line().await?.is_some() {
        if tx.send(()).is_err() {
            break;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub struct Timing {
    pub reminders: Duration,
    pub calendar: Duration,
    pub calendar_wait: Duration,
    pub export: Duration,
    pub backup: Duration,
    pub snapshots: Duration,
}

impl Timing {
    pub const PRODUCTION: Timing = Timing {
        reminders: Duration::from_mins(5),
        calendar: Duration::from_mins(15),
        calendar_wait: Duration::from_secs(10),
        export: Duration::from_hours(1),
        backup: Duration::from_hours(1),
        snapshots: Duration::from_secs(10),
    };
}

fn snapshot_count(dir: &Path) -> usize {
    std::fs::read_dir(dir.join("snapshots")).map_or(0, Iterator::count)
}

/// The fixed-interval timers; `next` waits for the first one due and names its step.
struct Ticks {
    reminders: Interval,
    calendar: Interval,
    export: Interval,
    backup: Interval,
    snapshots: Interval,
    seen: usize,
    dir: PathBuf,
}

impl Ticks {
    fn new(t: Timing, dir: PathBuf) -> Ticks {
        let every = |d: Duration| interval_at(Instant::now() + d, d);
        Ticks {
            reminders: every(t.reminders),
            calendar: every(t.calendar),
            export: every(t.export),
            backup: every(t.backup),
            snapshots: every(t.snapshots),
            seen: snapshot_count(&dir),
            dir,
        }
    }

    async fn next(&mut self) -> Step {
        loop {
            tokio::select! {
                _ = self.reminders.tick() => return Step::Reminders,
                _ = self.calendar.tick() => return Step::Gcal,
                _ = self.export.tick() => return Step::Export,
                _ = self.backup.tick() => return Step::Backup,
                _ = self.snapshots.tick() => {
                    let now = snapshot_count(&self.dir);
                    let grew = now > self.seen;
                    self.seen = now;
                    if grew {
                        return Step::Export;
                    }
                }
            }
        }
    }
}

/// Everything at start, then the timers: reminders on each helper change line and every interval, the
/// calendar `calendar_wait` after the last change message and every interval, export every interval and
/// after a new MCP or CLI snapshot file, backups every interval. Never returns.
pub async fn run<S: Steps>(
    syncer: Arc<Syncer<S>>,
    t: Timing,
    backup_dir: PathBuf,
    mut calendar_changes: UnboundedReceiver<()>,
    mut reminder_changes: UnboundedReceiver<()>,
) {
    syncer.sync_all().await.ok();
    let mut ticks = Ticks::new(t, backup_dir);
    let mut calendar_due: Option<Instant> = None;
    loop {
        tokio::select! {
            step = ticks.next() => syncer.step(step).await,
            Some(()) = reminder_changes.recv() => syncer.step(Step::Reminders).await,
            Some(()) = calendar_changes.recv() => calendar_due = Some(Instant::now() + t.calendar_wait),
            () = sleep_until(calendar_due.unwrap_or_else(Instant::now)), if calendar_due.is_some() => {
                calendar_due = None;
                syncer.step(Step::Gcal).await;
            }
        }
    }
}
