use crate::{ChangeSource, Db, Error, Pin, PinId, PinKind, ResolvedPin, Result, rows, settings};

type PinRow = (i64, String, String, Option<String>, i64);

fn kind_str(kind: PinKind) -> &'static str {
    match kind {
        PinKind::Issue => "issue",
        PinKind::Project => "project",
        PinKind::View => "view",
        PinKind::Cycle => "cycle",
        PinKind::Note => "note",
        PinKind::Url => "url",
    }
}

fn pin_from_row((id, kind, target, title, position): PinRow) -> Result<Pin> {
    let kind = match kind.as_str() {
        "issue" => PinKind::Issue,
        "project" => PinKind::Project,
        "view" => PinKind::View,
        "cycle" => PinKind::Cycle,
        "note" => PinKind::Note,
        "url" => PinKind::Url,
        other => return Err(Error::Invalid(format!("pin kind {other}"))),
    };
    Ok(Pin {
        id: PinId(id),
        kind,
        target,
        title,
        position: i32::try_from(position).map_err(|e| Error::Invalid(e.to_string()))?,
    })
}

/// Web links may be http(s) or Obsidian; notes must be Obsidian links.
fn validate(kind: PinKind, target: &str) -> Result<()> {
    let lower = target.to_ascii_lowercase();
    let ok = match kind {
        PinKind::Url => {
            lower.starts_with("http://")
                || lower.starts_with("https://")
                || lower.starts_with("obsidian://")
        }
        PinKind::Note => lower.starts_with("obsidian://"),
        _ => !target.trim().is_empty(),
    };
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "pin target {target:?} for {}",
            kind_str(kind)
        )))
    }
}

async fn find(db: &Db, kind: PinKind, target: &str) -> Result<Option<Pin>> {
    sqlx::query_as::<_, PinRow>(
        "SELECT id, kind, target, title, position FROM pins WHERE kind = ? AND target = ?",
    )
    .bind(kind_str(kind))
    .bind(target)
    .fetch_optional(&db.pool)
    .await?
    .map(pin_from_row)
    .transpose()
}

/// Pins or unpins every target in one transaction, so it is one undo step. Pinned targets are skipped.
///
/// # Errors
///
/// Fails on a database error or an invalid target; then nothing changes.
pub async fn set(db: &Db, kind: PinKind, targets: &[String], pinned: bool) -> Result<()> {
    for t in targets {
        validate(kind, t)?;
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    for t in targets {
        if pinned {
            let added = sqlx::query(
                "INSERT INTO pins (kind, target, title, position)
                 VALUES (?, ?, NULL, (SELECT COALESCE(MAX(position) + 1, 0) FROM pins))
                 ON CONFLICT (kind, target) DO NOTHING",
            )
            .bind(kind_str(kind))
            .bind(t)
            .execute(&mut *tx)
            .await?;
            if added.rows_affected() == 1 {
                rows::created(&mut tx, ChangeSource::Ui, "pins", added.last_insert_rowid()).await?;
            }
        } else {
            let id: Option<i64> =
                sqlx::query_scalar("SELECT id FROM pins WHERE kind = ? AND target = ?")
                    .bind(kind_str(kind))
                    .bind(t)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some(id) = id {
                let before = rows::by_key(&mut tx, "pins", id).await?;
                sqlx::query("DELETE FROM pins WHERE id = ?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                rows::changed(&mut tx, ChangeSource::Ui, before).await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Idempotent: pinning the same `(kind, target)` twice returns the existing pin.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn pin(db: &Db, kind: PinKind, target: &str, title: Option<String>) -> Result<Pin> {
    pin_as(db, ChangeSource::Ui, kind, target, title).await
}

/// Like [`pin`], recording `src` as the change source (the MCP server pins as the agent).
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn pin_as(
    db: &Db,
    src: ChangeSource,
    kind: PinKind,
    target: &str,
    title: Option<String>,
) -> Result<Pin> {
    validate(kind, target)?;
    if let Some(existing) = find(db, kind, target).await? {
        return Ok(existing);
    }
    let title = title.filter(|t| !t.trim().is_empty());
    // ON CONFLICT keeps this safe if another writer pins the same target between the two queries.
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let added = sqlx::query(
        "INSERT INTO pins (kind, target, title, position)
         VALUES (?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM pins))
         ON CONFLICT (kind, target) DO NOTHING",
    )
    .bind(kind_str(kind))
    .bind(target)
    .bind(title)
    .execute(&mut *tx)
    .await?;
    if added.rows_affected() == 1 {
        rows::created(&mut tx, src, "pins", added.last_insert_rowid()).await?;
    }
    tx.commit().await?;
    find(db, kind, target)
        .await?
        .ok_or_else(|| Error::NotFound(format!("pin {target}")))
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn unpin(db: &Db, id: PinId) -> Result<()> {
    unpin_as(db, ChangeSource::Ui, id).await
}

/// Like [`unpin`], recording `src` as the change source.
///
/// # Errors
///
/// Fails on a database error or when the pin does not exist.
pub async fn unpin_as(db: &Db, src: ChangeSource, id: PinId) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "pins", id.0).await?;
    let done = sqlx::query("DELETE FROM pins WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound(format!("pin {}", id.0)));
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn reorder(db: &Db, ids_in_order: &[PinId]) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut have: Vec<i64> = sqlx::query_scalar("SELECT id FROM pins")
        .fetch_all(&mut *tx)
        .await?;
    let mut want: Vec<i64> = ids_in_order.iter().map(|id| id.0).collect();
    have.sort_unstable();
    want.sort_unstable();
    // Pin ids are unique, so equal sorted lists mean every pin appears exactly once.
    if want != have {
        return Err(Error::Invalid(
            "reorder must list every current pin exactly once".into(),
        ));
    }
    let before = rows::all(&mut tx, "pins").await?;
    for (index, id) in ids_in_order.iter().enumerate() {
        let position = i64::try_from(index).map_err(|e| Error::Invalid(e.to_string()))?;
        sqlx::query("UPDATE pins SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, ChangeSource::Ui, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Label, archived flag and route for a pin's target, or `None` when the target is gone.
async fn resolve(db: &Db, prefix: &str, pin: &Pin) -> Result<Option<(String, bool, String)>> {
    let t = pin.target.as_str();
    Ok(match pin.kind {
        PinKind::Issue => {
            // The number alone identifies the issue, so a changed issue prefix does not orphan the pin.
            let number = t.rsplit_once('-').and_then(|(_, n)| n.parse::<i64>().ok());
            match number {
                None => None,
                Some(number) => sqlx::query_as::<_, (String, Option<String>)>(
                    "SELECT title, archived_at FROM issues WHERE number = ?",
                )
                .bind(number)
                .fetch_optional(&db.pool)
                .await?
                .map(|(title, archived)| {
                    (
                        title,
                        archived.is_some(),
                        format!("/issue/{prefix}-{number}"),
                    )
                }),
            }
        }
        PinKind::Project => sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT name, archived_at FROM projects WHERE slug = ?",
        )
        .bind(t)
        .fetch_optional(&db.pool)
        .await?
        .map(|(name, archived)| (name, archived.is_some(), format!("/project/{t}"))),
        PinKind::View => match t.parse::<i64>() {
            Err(_) => None,
            Ok(id) => sqlx::query_scalar::<_, String>("SELECT name FROM views WHERE id = ?")
                .bind(id)
                .fetch_optional(&db.pool)
                .await?
                .map(|name| (name, false, format!("/view/{id}"))),
        },
        PinKind::Cycle => match t.parse::<i64>() {
            Err(_) => None,
            Ok(n) => {
                sqlx::query_scalar::<_, Option<String>>("SELECT name FROM cycles WHERE number = ?")
                    .bind(n)
                    .fetch_optional(&db.pool)
                    .await?
                    .map(|name| {
                        (
                            name.unwrap_or_else(|| format!("Cycle {n}")),
                            false,
                            format!("/cycles#{n}"),
                        )
                    })
            }
        },
        // Notes and links have no table to check, so they always resolve.
        PinKind::Note | PinKind::Url => Some((t.to_owned(), false, t.to_owned())),
    })
}

/// Drops pins whose target no longer exists.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list_resolved(db: &Db) -> Result<Vec<ResolvedPin>> {
    let prefix = settings::get::<String>(db, "issue_prefix")
        .await?
        .ok_or_else(|| Error::NotFound("setting issue_prefix".into()))?;
    let rows = sqlx::query_as::<_, PinRow>(
        "SELECT id, kind, target, title, position FROM pins ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let pin = pin_from_row(row)?;
        match resolve(db, &prefix, &pin).await? {
            Some((name, archived, route)) => out.push(ResolvedPin {
                label: pin.title.clone().unwrap_or(name),
                archived,
                route,
                pin,
            }),
            None => {
                sqlx::query("DELETE FROM pins WHERE id = ?")
                    .bind(pin.id.0)
                    .execute(&db.pool)
                    .await?;
            }
        }
    }
    Ok(out)
}
