use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Db, Error, Issue, IssueId, IssuePatch, Relation, RelationKind, Result, StatusId,
    history::{self, Edit},
    issues, relation_kinds, rows,
};

fn to_json(rel: &Relation) -> Result<serde_json::Value> {
    serde_json::to_value(rel).map_err(|e| Error::Invalid(e.to_string()))
}

/// Marks `id` a duplicate of `of`: a duplicate relation from `id` to `of`, and status Duplicate on `id`.
/// Both writes belong to one command call, so one undo step reverts both.
///
/// # Errors
///
/// `Invalid` when no status is called Duplicate, or when the two issues are the same; `NotFound` for an unknown issue.
pub async fn mark_duplicate(db: &Db, id: IssueId, of: IssueId, src: ChangeSource) -> Result<Issue> {
    let status: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM statuses WHERE lower(name) = 'duplicate' ORDER BY position, id LIMIT 1",
    )
    .fetch_optional(&db.pool)
    .await?;
    let status = status
        .ok_or_else(|| Error::Invalid("no status called Duplicate; add one in Settings".into()))?;
    add(
        db,
        Relation {
            from: id,
            to: of,
            kind: RelationKind::new("duplicate"),
        },
        src,
    )
    .await?;
    let patch = IssuePatch {
        status_id: Some(StatusId(status)),
        ..IssuePatch::default()
    };
    issues::update(db, id, patch, src).await
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
    let rel = Relation {
        kind: relation_kinds::resolve(tx, &rel.kind).await?,
        ..rel
    };
    let (from, to, kind) = (rel.from.0, rel.to.0, rel.kind.0.as_str());
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
        rows::created(tx, src, "relations", done.last_insert_rowid()).await?;
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
    let mut tx = db.pool.begin().await?;
    let rel = Relation {
        kind: relation_kinds::resolve(&mut tx, &rel.kind).await?,
        ..rel
    };
    let (from, to, kind) = (rel.from.0, rel.to.0, rel.kind.0.as_str());
    let id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM relations WHERE from_id = ? AND to_id = ? AND kind = ?")
            .bind(from)
            .bind(to)
            .bind(kind)
            .fetch_optional(&mut *tx)
            .await?;
    let before = rows::by_key(&mut tx, "relations", id.unwrap_or(0)).await?;
    let done = sqlx::query!(
        "DELETE FROM relations WHERE from_id = ? AND to_id = ? AND kind = ?",
        from,
        to,
        kind
    )
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 1 {
        rows::changed(&mut tx, src, before).await?;
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
    Ok(rows
        .into_iter()
        .map(|r| Relation {
            from: IssueId(r.from_id),
            to: IssueId(r.to_id),
            kind: RelationKind(r.kind),
        })
        .collect())
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
            kind: RelationKind::new("blocks"),
        })
        .collect())
}
