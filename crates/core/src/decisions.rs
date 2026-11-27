//! Decisions: questions with options that the owner answers. Raised by the UI, the CLI, an agent or (later) an
//! integration. Every write is a row snapshot (rows.rs), so undo covers decisions and their links.

use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Decision, DecisionId, DecisionInput, DecisionStatus, Error, IssueId, Result,
    rows,
};

const COLS: &str = "id, question, context, options, recommendation, mode, status, answer, note, source, external_id, created_at, answered_at";

fn bad<T: ToString + ?Sized>(e: &T) -> Error {
    Error::Invalid(e.to_string())
}

fn word<T: serde::Serialize>(v: T) -> Result<String> {
    match serde_json::to_value(v).map_err(|e| bad(&e))? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(bad(&format!("not a word: {other}"))),
    }
}

fn parse<T: serde::de::DeserializeOwned>(s: String) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s)).map_err(|e| bad(&e))
}

async fn from_row(tx: &mut SqliteConnection, r: &SqliteRow) -> Result<Decision> {
    let id: i64 = r.try_get("id")?;
    let issue_ids = sqlx::query_scalar(
        "SELECT issue_id FROM decision_issues WHERE decision_id = ? ORDER BY issue_id",
    )
    .bind(id)
    .fetch_all(tx)
    .await?
    .into_iter()
    .map(IssueId)
    .collect();
    Ok(Decision {
        id: DecisionId(id),
        question: r.try_get("question")?,
        context: r.try_get("context")?,
        options: serde_json::from_str(&r.try_get::<String, _>("options")?).map_err(|e| bad(&e))?,
        recommendation: r.try_get("recommendation")?,
        mode: parse(r.try_get("mode")?)?,
        status: parse(r.try_get("status")?)?,
        answer: r.try_get("answer")?,
        note: r.try_get("note")?,
        source: r.try_get("source")?,
        external_id: r.try_get("external_id")?,
        created_at: r.try_get("created_at")?,
        answered_at: r.try_get("answered_at")?,
        issue_ids,
    })
}

async fn fetch(tx: &mut SqliteConnection, id: DecisionId) -> Result<Decision> {
    let row = sqlx::query(&format!("SELECT {COLS} FROM decisions WHERE id = ?"))
        .bind(id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("decision {}", id.0)))?;
    from_row(tx, &row).await
}

fn check(input: &DecisionInput) -> Result<()> {
    if input.question.trim().is_empty() {
        return Err(bad("a decision needs a question"));
    }
    let mut keys = std::collections::HashSet::new();
    for o in &input.options {
        if o.key.trim().is_empty() || o.label.trim().is_empty() {
            return Err(bad("every option needs a key and a label"));
        }
        if !keys.insert(&o.key) {
            return Err(bad(&format!("option key {} is used twice", o.key)));
        }
    }
    if let Some(r) = &input.recommendation
        && !keys.contains(r)
    {
        return Err(bad(&format!(
            "recommendation {r} is not one of the options"
        )));
    }
    Ok(())
}

/// Decisions with `status` (all when `None`), blocking ones first, then newest first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db, status: Option<DecisionStatus>) -> Result<Vec<Decision>> {
    let status = status.map(word).transpose()?;
    let mut conn = db.pool.acquire().await?;
    let rows = sqlx::query(&format!(
        "SELECT {COLS} FROM decisions WHERE (?1 IS NULL OR status = ?1)
         ORDER BY mode = 'blocking' DESC, id DESC"
    ))
    .bind(status)
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(from_row(&mut conn, r).await?);
    }
    Ok(out)
}

/// # Errors
///
/// `NotFound` for an unknown id, or a database error.
pub async fn get(db: &Db, id: DecisionId) -> Result<Decision> {
    fetch(&mut *db.pool.acquire().await?, id).await
}

/// # Errors
///
/// `Invalid` for an empty question, bad option keys, a recommendation that is no option, or an
/// `external_id` already used by the same source; `NotFound` for an unknown issue.
pub async fn create(db: &Db, input: DecisionInput, src: ChangeSource) -> Result<Decision> {
    check(&input)?;
    let source = input.source.clone().unwrap_or_else(|| {
        match src {
            ChangeSource::Ui => "ui",
            ChangeSource::Agent => "agent",
            _ => "cli",
        }
        .to_owned()
    });
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query(
        "INSERT INTO decisions (question, context, options, recommendation, mode, source, external_id)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.question)
    .bind(&input.context)
    .bind(serde_json::to_string(&input.options).map_err(|e| bad(&e))?)
    .bind(&input.recommendation)
    .bind(word(input.mode)?)
    .bind(&source)
    .bind(&input.external_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => bad(&format!(
            "source {source} already raised a decision with that external id"
        )),
        _ => Error::Db(e),
    })?
    .last_insert_rowid();
    rows::created(&mut tx, src, "decisions", id).await?;
    for issue in &input.issue_ids {
        link_in(&mut tx, DecisionId(id), *issue, src).await?;
    }
    let decision = fetch(&mut tx, DecisionId(id)).await?;
    tx.commit().await?;
    Ok(decision)
}

/// Answers an open decision with exactly one of an option key or free text.
///
/// # Errors
///
/// `Invalid` when both or neither is given, the key is no option, or the decision is not open.
pub async fn answer(
    db: &Db,
    id: DecisionId,
    option: Option<String>,
    text: Option<String>,
    note: String,
    src: ChangeSource,
) -> Result<Decision> {
    if option.is_some() == text.is_some() || text.as_deref().is_some_and(|t| t.trim().is_empty()) {
        return Err(bad("answer with exactly one of an option or some text"));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let d = fetch(&mut tx, id).await?;
    if d.status != DecisionStatus::Open {
        return Err(bad(&format!("decision {} is not open", id.0)));
    }
    if let Some(key) = &option
        && d.options.iter().all(|o| &o.key != key)
    {
        return Err(bad(&format!("{key} is not one of the options")));
    }
    let answer = option.or(text);
    finish_commit(tx, d, "answered", answer, note, src).await
}

/// Closes an open decision without an answer (the question no longer applies).
///
/// # Errors
///
/// `Invalid` when it is not open; `NotFound` for an unknown id.
pub async fn withdraw(db: &Db, id: DecisionId, src: ChangeSource) -> Result<Decision> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let d = fetch(&mut tx, id).await?;
    if d.status != DecisionStatus::Open {
        return Err(bad(&format!("decision {} is not open", id.0)));
    }
    finish_commit(tx, d, "withdrawn", None, String::new(), src).await
}

async fn finish(
    tx: &mut SqliteConnection,
    d: Decision,
    status: &str,
    answer: Option<String>,
    note: String,
    src: ChangeSource,
) -> Result<Decision> {
    let before = rows::by_key(tx, "decisions", d.id.0).await?;
    sqlx::query(
        "UPDATE decisions SET status = ?, answer = ?, note = ?, answered_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(status)
    .bind(answer)
    .bind(note)
    .bind(d.id.0)
    .execute(&mut *tx)
    .await?;
    rows::changed(tx, src, before).await?;
    fetch(tx, d.id).await
}

async fn finish_commit(
    mut tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    d: Decision,
    status: &str,
    answer: Option<String>,
    note: String,
    src: ChangeSource,
) -> Result<Decision> {
    let out = finish(&mut tx, d, status, answer, note, src).await?;
    tx.commit().await?;
    Ok(out)
}

async fn link_in(
    tx: &mut SqliteConnection,
    decision: DecisionId,
    issue: IssueId,
    src: ChangeSource,
) -> Result<()> {
    let done =
        sqlx::query("INSERT OR IGNORE INTO decision_issues (decision_id, issue_id) VALUES (?, ?)")
            .bind(decision.0)
            .bind(issue.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(d) if d.is_foreign_key_violation() => {
                    Error::NotFound(format!("issue {}", issue.0))
                }
                _ => Error::Db(e),
            })?;
    if done.rows_affected() == 1 {
        rows::created(tx, src, "decision_issues", done.last_insert_rowid()).await?;
    }
    Ok(())
}

/// Idempotent: linking an already linked pair succeeds and records nothing.
///
/// # Errors
///
/// `NotFound` for an unknown decision or issue, or a database error.
pub async fn link(db: &Db, decision: DecisionId, issue: IssueId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, decision).await?;
    link_in(&mut tx, decision, issue, src).await?;
    tx.commit().await?;
    Ok(())
}

/// Removing a link that does not exist is not an error.
///
/// # Errors
///
/// Fails on a database error.
pub async fn unlink(
    db: &Db,
    decision: DecisionId,
    issue: IssueId,
    src: ChangeSource,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM decision_issues WHERE decision_id = ? AND issue_id = ?")
            .bind(decision.0)
            .bind(issue.0)
            .fetch_optional(&mut *tx)
            .await?;
    let before = rows::by_key(&mut tx, "decision_issues", id.unwrap_or(0)).await?;
    sqlx::query("DELETE FROM decision_issues WHERE decision_id = ? AND issue_id = ?")
        .bind(decision.0)
        .bind(issue.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
