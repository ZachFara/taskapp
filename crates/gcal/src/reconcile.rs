use std::collections::{HashMap, HashSet};

use chrono::{Duration, Utc};
use sqlx::Row;
use taskapp_core::{
    Db, Error, Layout, Result, SyncState, ViewDef, blocks, events, projects, settings, statuses,
    views,
};

use crate::{
    auth,
    client::{self, Google, Remote, flag},
    planner::{self, DesiredEvent, EventOp, ManagedEvent},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    pub created: u32,
    pub updated: u32,
    pub deleted: u32,
}

/// # Errors
///
/// Fails when the calendar request fails; an expired or revoked token sets the visible error state first.
pub async fn reconcile(db: &Db) -> Result<ReconcileReport> {
    if !settings::get::<bool>(db, "gcal_enabled")
        .await?
        .unwrap_or(false)
    {
        return Ok(ReconcileReport::default());
    }
    let g = Google::production()?;
    let token = flag(db, auth::load_token()).await?;
    reconcile_with(db, &g, &token).await
}

struct Known {
    etag: Option<String>,
    hash: String,
}

/// What every call against the calendar needs.
struct Ctx<'a> {
    db: &'a Db,
    g: &'a Google,
    token: String,
    cal: String,
}

/// `reconcile` against explicit endpoints and refresh token.
///
/// # Errors
///
/// As `reconcile`.
pub async fn reconcile_with(db: &Db, g: &Google, refresh_token: &str) -> Result<ReconcileReport> {
    flag(db, run(db, g, refresh_token).await).await
}

async fn run(db: &Db, g: &Google, refresh_token: &str) -> Result<ReconcileReport> {
    let cal: String = settings::get(db, "gcal_calendar_id")
        .await?
        .ok_or_else(|| Error::Invalid("choose a Google calendar in Settings first".into()))?;
    let token = client::access_token(g, refresh_token).await?;
    let remote = client::list_managed(g, &token, &cal).await?;
    let ctx = Ctx { db, g, token, cal };
    let mut report = ReconcileReport::default();
    let existing = ctx.existing(&remote, &mut report).await?;
    let etags: HashMap<&str, &str> = remote
        .iter()
        .map(|r| (r.id.as_str(), r.etag.as_str()))
        .collect();
    for op in planner::plan(&desired(db).await?, &existing) {
        ctx.apply(op, &etags, &mut report).await?;
    }
    client::set_state(db, SyncState::Ok { at: Utc::now() }).await?;
    Ok(report)
}

impl Ctx<'_> {
    /// Rows for this calendar only; rows for any other calendar id are left alone.
    async fn known(&self) -> Result<HashMap<String, Known>> {
        Ok(sqlx::query(
            "SELECT google_event_id, etag, last_synced_hash FROM gcal_events WHERE calendar_id = ?",
        )
        .bind(&self.cal)
        .fetch_all(&self.db.pool)
        .await?
        .into_iter()
        .map(|r| {
            (
                r.get("google_event_id"),
                Known {
                    etag: r.get("etag"),
                    hash: r.get("last_synced_hash"),
                },
            )
        })
        .collect())
    }

    /// The events the calendar holds, paired with what we last wrote. Rows whose event vanished are
    /// dropped, and a second event under one managed key is deleted.
    async fn existing(
        &self,
        remote: &[Remote],
        report: &mut ReconcileReport,
    ) -> Result<Vec<ManagedEvent>> {
        let mut rows = self.known().await?;
        let live: HashSet<&str> = remote.iter().map(|r| r.id.as_str()).collect();
        for id in rows.keys().filter(|id| !live.contains(id.as_str())) {
            self.forget(id).await?;
        }
        let mut existing: Vec<ManagedEvent> = Vec::new();
        for r in remote {
            if existing.iter().any(|e| e.managed_key == r.managed_key) {
                client::delete(self.g, &self.token, &self.cal, &r.id).await?;
                report.deleted += 1;
                continue;
            }
            // An event edited by hand has a new etag, so its stored hash no longer vouches for it.
            let known = rows
                .remove(&r.id)
                .filter(|row| row.etag.as_deref() == Some(&r.etag));
            existing.push(ManagedEvent {
                google_event_id: r.id.clone(),
                managed_key: r.managed_key.clone(),
                last_synced_hash: known.map_or_default(|k| k.hash),
            });
        }
        Ok(existing)
    }

    async fn forget(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM gcal_events WHERE google_event_id = ?")
            .bind(id)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    async fn apply(
        &self,
        op: EventOp,
        etags: &HashMap<&str, &str>,
        report: &mut ReconcileReport,
    ) -> Result<()> {
        let (g, token, cal) = (self.g, &self.token, &self.cal);
        match op {
            EventOp::Create(e) => {
                let (id, etag) = client::insert(g, token, cal, &e).await?;
                self.remember(&id, &etag, &e).await?;
                report.created += 1;
            }
            EventOp::Update {
                google_event_id,
                event,
            } => {
                let etag = etags.get(google_event_id.as_str()).copied();
                let (id, etag) =
                    match client::update(g, token, cal, &google_event_id, etag, &event).await? {
                        Some(stored) => stored,
                        // Deleted by hand since the listing: put it back.
                        None => client::insert(g, token, cal, &event).await?,
                    };
                self.remember(&id, &etag, &event).await?;
                report.updated += 1;
            }
            EventOp::Delete { google_event_id } => {
                client::delete(g, token, cal, &google_event_id).await?;
                self.forget(&google_event_id).await?;
                report.deleted += 1;
            }
        }
        Ok(())
    }

    async fn remember(&self, id: &str, etag: &str, e: &DesiredEvent) -> Result<()> {
        // REPLACE also clears a stale row that held this managed key under another event id.
        sqlx::query(
            "INSERT OR REPLACE INTO gcal_events (google_event_id, calendar_id, managed_key, etag, last_synced_hash)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&self.cal)
        .bind(&e.managed_key)
        .bind(etag)
        .bind(planner::hash(e))
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }
}

async fn desired(db: &Db) -> Result<Vec<DesiredEvent>> {
    let def = ViewDef {
        filter: None,
        sorts: vec![],
        group_by: None,
        layout: Layout::List,
        columns: vec![],
        include_archived: false,
        ..ViewDef::default()
    };
    let issues = views::query(db, &def).await?;
    // SQLite julianday() cannot parse chrono's MIN/MAX years, so an unbounded range matches nothing.
    let now = Utc::now();
    let (from, to) = (now - Duration::days(30), now + Duration::days(365));
    let blocks = blocks::blocks_between(db, from, to).await?;
    // shortcut: an unset `timezone` setting means UTC, so 08:00 alerts fire at UTC 08:00; upgrade when settings own a zone.
    let tz: String = settings::get(db, "timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let prefix: String = settings::get(db, "issue_prefix")
        .await?
        .unwrap_or_else(|| "ABC".into());
    let mut out = planner::desired(
        &issues,
        &statuses::list(db).await?,
        &projects::list(db).await?,
        &blocks,
        &tz,
        &prefix,
    );
    let (from, to) = (from.date_naive(), to.date_naive());
    out.extend(planner::event_events(
        &events::list(db, Some(from), Some(to)).await?,
        &issues,
        &tz,
    ));
    Ok(out)
}
