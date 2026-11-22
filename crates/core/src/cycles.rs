use chrono::NaiveDate;
use serde_json::{Value, json};
use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Cycle, CycleId, Db, Error, Result,
    history::{self, Edit},
    palette::{self, Look},
};

/// Icon and color are checked the same way on create and update.
fn check_look(icon: Option<&str>, color: Option<&str>) -> Result<Option<String>> {
    if let Some(icon) = icon {
        palette::check_icon(icon)?;
    }
    color
        .map(|c| palette::key_for(c).map(str::to_owned))
        .transpose()
}

/// Cycles share days at both ends (a cycle ending on the 14th overlaps one starting on the 14th).
const OVERLAPS: &str =
    "SELECT count(*) FROM cycles WHERE id != ? AND start_date <= ? AND end_date >= ?";

fn check_dates(start: NaiveDate, end: NaiveDate) -> Result<()> {
    if end < start {
        return Err(Error::Invalid(format!(
            "cycle ends {end} before it starts {start}"
        )));
    }
    Ok(())
}

/// Checked after the write, inside the caller's transaction: the write takes the database's
/// write lock, so two overlapping creates cannot both pass this check.
async fn ensure_no_overlap(
    tx: &mut SqliteConnection,
    id: i64,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<()> {
    let clashes: i64 = sqlx::query_scalar(OVERLAPS)
        .bind(id)
        .bind(end)
        .bind(start)
        .fetch_one(&mut *tx)
        .await?;
    if clashes > 0 {
        return Err(Error::Conflict(format!(
            "cycle {start} to {end} overlaps another cycle"
        )));
    }
    Ok(())
}

/// Records one field's change, skipping unchanged values.
async fn edit(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    id: i64,
    field: &str,
    old: Value,
    new: Value,
) -> Result<()> {
    if old == new {
        return Ok(());
    }
    let edit = Edit {
        field,
        old: Some(&old),
        new: Some(&new),
    };
    history::record(tx, src, "cycle", &id.to_string(), &edit).await
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(db: &Db) -> Result<Vec<Cycle>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", number AS "number!: i64", name,
                  start_date AS "start_date!: NaiveDate", end_date AS "end_date!: NaiveDate",
                  icon, color
           FROM cycles ORDER BY number"#
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Cycle {
                id: CycleId(r.id),
                number: u32::try_from(r.number).map_err(|e| Error::Invalid(e.to_string()))?,
                name: r.name,
                start_date: r.start_date,
                end_date: r.end_date,
                icon: r.icon,
                color: r.color,
            })
        })
        .collect()
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(
    db: &Db,
    name: Option<String>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    look: Look,
    src: ChangeSource,
) -> Result<Cycle> {
    check_dates(start_date, end_date)?;
    let Look { icon, color } = look;
    let color = check_look(icon.as_deref(), color.as_deref())?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    // The number is taken in the same statement as the insert, so it cannot repeat.
    let id = sqlx::query!(
        "INSERT INTO cycles (number, name, start_date, end_date, icon, color)
         VALUES ((SELECT coalesce(max(number), 0) + 1 FROM cycles), ?, ?, ?, ?, ?)",
        name,
        start_date,
        end_date,
        icon,
        color
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    ensure_no_overlap(&mut tx, id, start_date, end_date).await?;
    let number = sqlx::query_scalar!(
        r#"SELECT number AS "number!: i64" FROM cycles WHERE id = ?"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    let created = json!(start_date.to_string() + " to " + &end_date.to_string());
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "cycle", &id.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(Cycle {
        id: CycleId(id),
        number: u32::try_from(number).map_err(|e| Error::Invalid(e.to_string()))?,
        name,
        start_date,
        end_date,
        icon,
        color,
    })
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(db: &Db, mut cycle: Cycle, src: ChangeSource) -> Result<Cycle> {
    check_dates(cycle.start_date, cycle.end_date)?;
    cycle.color = check_look(cycle.icon.as_deref(), cycle.color.as_deref())?;
    let id = cycle.id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = sqlx::query!(
        r#"SELECT name, start_date AS "start_date!: NaiveDate", end_date AS "end_date!: NaiveDate",
                  icon, color
           FROM cycles WHERE id = ?"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("cycle {id}")))?;
    edit(&mut tx, src, id, "name", json!(old.name), json!(cycle.name)).await?;
    edit(&mut tx, src, id, "icon", json!(old.icon), json!(cycle.icon)).await?;
    edit(
        &mut tx,
        src,
        id,
        "color",
        json!(old.color),
        json!(cycle.color),
    )
    .await?;
    edit(
        &mut tx,
        src,
        id,
        "start_date",
        json!(old.start_date),
        json!(cycle.start_date),
    )
    .await?;
    edit(
        &mut tx,
        src,
        id,
        "end_date",
        json!(old.end_date),
        json!(cycle.end_date),
    )
    .await?;
    sqlx::query!(
        "UPDATE cycles SET name = ?, start_date = ?, end_date = ?, icon = ?, color = ? WHERE id = ?",
        cycle.name,
        cycle.start_date,
        cycle.end_date,
        cycle.icon,
        cycle.color,
        id
    )
    .execute(&mut *tx)
    .await?;
    ensure_no_overlap(&mut tx, id, cycle.start_date, cycle.end_date).await?;
    tx.commit().await?;
    Ok(cycle)
}

/// Issues keep existing and lose the cycle.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(db: &Db, id: CycleId, src: ChangeSource) -> Result<()> {
    let cid = id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let exists: i64 = sqlx::query_scalar("SELECT count(*) FROM cycles WHERE id = ?")
        .bind(cid)
        .fetch_one(&mut *tx)
        .await?;
    if exists == 0 {
        return Err(Error::NotFound(format!("cycle {cid}")));
    }
    let affected: Vec<i64> = sqlx::query_scalar("SELECT id FROM issues WHERE cycle_id = ?")
        .bind(cid)
        .fetch_all(&mut *tx)
        .await?;
    for issue_id in affected {
        let field = "cycle_id";
        let old = json!(cid);
        let edit = Edit {
            field,
            old: Some(&old),
            new: None,
        };
        history::record(&mut tx, src, "issue", &issue_id.to_string(), &edit).await?;
    }
    sqlx::query!("UPDATE issues SET cycle_id = NULL WHERE cycle_id = ?", cid)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM cycles WHERE id = ?", cid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
