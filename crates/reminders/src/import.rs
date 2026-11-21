use std::path::Path;

use chrono::{NaiveDate, Utc};
use taskapp_core::{
    ChangeSource, Db, IssuePatch, Result, SyncState, SyncStatus, issues, quickadd, settings,
};

use crate::helper::{self, Reminder};

pub struct ImportReport {
    pub imported: u32,
    pub skipped_duplicates: u32,
    pub failed: Vec<String>,
}

/// Imports the configured list, then deletes every imported reminder (including ones whose
/// delete failed on an earlier run). A helper failure sets `sync_status.reminders` to Error.
///
/// # Errors
///
/// Fails when the helper cannot run or the database cannot be written.
pub async fn import(db: &Db, helper: &Path, today: NaiveDate, tz: &str) -> Result<ImportReport> {
    let list = settings::get::<String>(db, "reminders_list_id")
        .await?
        .unwrap_or_default();
    if list.is_empty() {
        return Ok(ImportReport {
            imported: 0,
            skipped_duplicates: 0,
            failed: vec![],
        });
    }
    let result = run(db, helper, &list, today, tz).await;
    let state = match &result {
        Ok(_) => SyncState::Ok { at: Utc::now() },
        Err(e) => SyncState::Error {
            at: Utc::now(),
            message: e.to_string(),
        },
    };
    let mut status = settings::get::<SyncStatus>(db, "sync_status")
        .await?
        .unwrap_or(SyncStatus {
            reminders: SyncState::Off,
            gcal: SyncState::Off,
            export: SyncState::Off,
            backup: SyncState::Off,
        });
    status.reminders = state;
    settings::set(db, "sync_status", &status).await?;
    result
}

async fn run(
    db: &Db,
    helper: &Path,
    list: &str,
    today: NaiveDate,
    tz: &str,
) -> Result<ImportReport> {
    let mut report = ImportReport {
        imported: 0,
        skipped_duplicates: 0,
        failed: vec![],
    };
    for r in helper::fetch(helper, list).await? {
        import_one(db, r, today, tz, &mut report).await?;
    }
    delete_pending(db, helper, list).await?;
    Ok(report)
}

async fn import_one(
    db: &Db,
    r: Reminder,
    today: NaiveDate,
    tz: &str,
    report: &mut ImportReport,
) -> Result<()> {
    let logged = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "n!: i64" FROM import_log WHERE external_id = ?"#,
        r.external_id
    )
    .fetch_one(&db.pool)
    .await?;
    if logged > 0 {
        report.skipped_duplicates += 1;
        return Ok(());
    }
    if r.title.trim().is_empty() {
        report
            .failed
            .push(format!("{}: empty title", r.external_id));
        return Ok(());
    }
    let src = ChangeSource::RemindersImport;
    let new = match quickadd::new_issue(db, &r.title, today, tz).await {
        Ok(n) => n,
        Err(e) => {
            report.failed.push(format!("{}: {e}", r.external_id));
            return Ok(());
        }
    };
    // One transaction, so a crash cannot leave an issue without its log row (a duplicate on re-import).
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let issue = match issues::create_in(&mut tx, new, src).await {
        Ok(i) => i,
        Err(e) => {
            report.failed.push(format!("{}: {e}", r.external_id));
            return Ok(());
        }
    };
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO import_log (external_id, source, issue_id, imported_at) VALUES (?, 'reminders', ?, ?)",
        r.external_id,
        issue.id.0,
        now
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    report.imported += 1;
    let notes = r.notes.filter(|n| !n.trim().is_empty());
    if notes.is_some() || r.due.is_some() {
        let patch = IssuePatch {
            description: notes,
            due: r.due.map(Some),
            ..IssuePatch::default()
        };
        if let Err(e) = issues::update(db, issue.id, patch, src).await {
            report
                .failed
                .push(format!("{}: notes or due not applied: {e}", r.external_id));
        }
    }
    Ok(())
}

/// One helper call for every logged reminder still in Reminders; ids that are already gone count as deleted.
async fn delete_pending(db: &Db, helper: &Path, list: &str) -> Result<()> {
    let pending = sqlx::query_scalar!(
        r#"SELECT external_id AS "external_id!: String" FROM import_log
           WHERE source = 'reminders' AND source_deleted = 0"#
    )
    .fetch_all(&db.pool)
    .await?;
    if pending.is_empty() {
        return Ok(());
    }
    for id in helper::delete(helper, list, &pending).await? {
        sqlx::query!(
            "UPDATE import_log SET source_deleted = 1 WHERE external_id = ?",
            id
        )
        .execute(&db.pool)
        .await?;
    }
    Ok(())
}
