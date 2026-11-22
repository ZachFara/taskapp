use sqlx::SqliteConnection;

use crate::history::{self, Edit};
use crate::{ChangeSource, Db, Error, IssueId, Relation, RelationKind, Result};

fn kind_str(k: RelationKind) -> &'static str {
    match k {
        RelationKind::Blocks => "blocks",
        RelationKind::Related => "related",
        RelationKind::Duplicate => "duplicate",
    }
}

fn parse_kind(s: &str) -> Result<RelationKind> {
    match s {
        "blocks" => Ok(RelationKind::Blocks),
        "related" => Ok(RelationKind::Related),
        "duplicate" => Ok(RelationKind::Duplicate),
        other => Err(Error::Invalid(format!("unknown relation kind {other}"))),
    }
}

fn to_json(rel: &Relation) -> Result<serde_json::Value> {
    serde_json::to_value(rel).map_err(|e| Error::Invalid(e.to_string()))
}

/// Idempotent: adding an existing relation succeeds and records nothing.
///
/// # Errors
///
/// `Invalid` for a self-relation, `NotFound` for an unknown issue, `Conflict` when a `Blocks` edge
/// would close a cycle.
pub async fn add(db: &Db, rel: Relation, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    add_in(&mut tx, rel, src).await?;
    tx.commit().await?;
    Ok(())
}

/// [`add`] inside the caller's transaction, which the caller commits.
pub(crate) async fn add_in(
    tx: &mut SqliteConnection,
    rel: Relation,
    src: ChangeSource,
) -> Result<()> {
    if rel.from == rel.to {
        return Err(Error::Invalid("an issue cannot relate to itself".into()));
    }
    let (from, to, kind) = (rel.from.0, rel.to.0, kind_str(rel.kind));
    // The cycle check and the insert are one statement, so a concurrent writer cannot slip an edge between them.
    let done = sqlx::query!(
        "WITH RECURSIVE reach(id) AS (
             SELECT ?2 UNION SELECT r.to_id FROM relations r JOIN reach ON r.from_id = reach.id WHERE r.kind = 'blocks')
         INSERT OR IGNORE INTO relations (from_id, to_id, kind)
         SELECT ?1, ?2, ?3 WHERE ?3 <> 'blocks' OR NOT EXISTS (SELECT 1 FROM reach WHERE id = ?1)",
        from,
        to,
        kind
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| missing_issue(e, &rel))?;
    if done.rows_affected() == 1 {
        let new = to_json(&rel)?;
        let edit = Edit {
            field: "relation",
            old: None,
            new: Some(&new),
        };
        history::record(tx, src, "issue", &from.to_string(), &edit).await?;
    } else if exists(tx, from, to, kind).await? {
        return Ok(());
    } else {
        return Err(Error::Conflict("would create a dependency cycle".into()));
    }
    Ok(())
}

fn missing_issue(e: sqlx::Error, rel: &Relation) -> Error {
    match &e {
        sqlx::Error::Database(d) if d.is_foreign_key_violation() => {
            Error::NotFound(format!("issue {} or {}", rel.from.0, rel.to.0))
        }
        _ => Error::Db(e),
    }
}

async fn exists(tx: &mut sqlx::SqliteConnection, from: i64, to: i64, kind: &str) -> Result<bool> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!: i64" FROM relations WHERE from_id = ? AND to_id = ? AND kind = ?"#,
        from,
        to,
        kind
    )
    .fetch_one(tx)
    .await?;
    Ok(n > 0)
}

/// Removing a relation that does not exist is not an error.
///
/// # Errors
///
/// Fails on a database error.
pub async fn remove(db: &Db, rel: Relation, src: ChangeSource) -> Result<()> {
    let (from, to, kind) = (rel.from.0, rel.to.0, kind_str(rel.kind));
    let mut tx = db.pool.begin().await?;
    let done = sqlx::query!(
        "DELETE FROM relations WHERE from_id = ? AND to_id = ? AND kind = ?",
        from,
        to,
        kind
    )
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 1 {
        let old = to_json(&rel)?;
        let edit = Edit {
            field: "relation",
            old: Some(&old),
            new: None,
        };
        history::record(&mut tx, src, "issue", &from.to_string(), &edit).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Relations the issue is either end of.
///
/// # Errors
///
/// Fails on a database error.
pub async fn for_issue(db: &Db, id: IssueId) -> Result<Vec<Relation>> {
    let rows = sqlx::query!(
        r#"SELECT from_id AS "from_id!: i64", to_id AS "to_id!: i64", kind AS "kind!: String"
           FROM relations WHERE from_id = ?1 OR to_id = ?1 ORDER BY id"#,
        id.0
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Relation {
                from: IssueId(r.from_id),
                to: IssueId(r.to_id),
                kind: parse_kind(&r.kind)?,
            })
        })
        .collect()
}

/// Every `Blocks` edge, for scheduling.
///
/// # Errors
///
/// Fails on a database error.
pub async fn all_blocking(db: &Db) -> Result<Vec<Relation>> {
    let rows = sqlx::query!(
        r#"SELECT from_id AS "from_id!: i64", to_id AS "to_id!: i64" FROM relations WHERE kind = 'blocks' ORDER BY id"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Relation {
            from: IssueId(r.from_id),
            to: IssueId(r.to_id),
            kind: RelationKind::Blocks,
        })
        .collect())
}
