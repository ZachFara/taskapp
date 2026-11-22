//! Saved issue templates: a body that prefills an issue, its properties and its sub-issues.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Db, Error, Issue, LabelId, NewIssue, ProjectId, Result, StatusId, TemplateId,
    history::{self, Edit},
    issues, palette, rows,
};

/// The fields a template prefills. Each is a default; an override on the issue wins.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct TemplateFields {
    pub description: Option<String>,
    pub status_id: Option<StatusId>,
    pub priority: Option<u8>,
    pub project_id: Option<ProjectId>,
    pub label_ids: Vec<LabelId>,
    pub estimate_minutes: Option<u32>,
    pub properties: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct SubIssue {
    pub title: String,
    pub fields: TemplateFields,
}

/// Stored as JSON. `title_prefix` is put before the issue's own title.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct TemplateBody {
    pub title_prefix: String,
    pub fields: TemplateFields,
    pub sub_issues: Vec<SubIssue>,
}

/// What create and update take: the whole template, replaced on update.
#[derive(Debug, Clone, PartialEq, Deserialize, specta::Type)]
pub struct TemplateInput {
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub body: TemplateBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
pub struct IssueTemplate {
    pub id: TemplateId,
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub body: TemplateBody,
}

/// The issue and sub-issues a template created. `skipped` names references the template
/// no longer resolves (a deleted label, project or status); those fields were left unset.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
pub struct AppliedTemplate {
    pub issue: Issue,
    pub sub_issues: Vec<Issue>,
    pub skipped: Vec<String>,
}

fn bad<E: std::fmt::Display>(e: E) -> Error {
    Error::Invalid(e.to_string())
}

fn validate(input: &mut TemplateInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(Error::Invalid("template name is empty".into()));
    }
    let slug_ok = !input.slug.is_empty()
        && input
            .slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !slug_ok {
        return Err(Error::Invalid(format!(
            "slug {} must be lowercase letters, digits or dashes",
            input.slug
        )));
    }
    if let Some(icon) = &input.icon {
        palette::check_icon(icon)?;
    }
    if let Some(color) = input.color.take() {
        input.color = Some(palette::key_for(&color)?.to_owned());
    }
    let fields =
        std::iter::once(&input.body.fields).chain(input.body.sub_issues.iter().map(|s| &s.fields));
    for f in fields {
        if f.priority.is_some_and(|p| p > 4) {
            return Err(Error::Invalid("template priority is not 0 to 4".into()));
        }
    }
    Ok(())
}

/// `/slug rest` at the start of quick-add text: the slug and the text after it.
pub(crate) fn split_slug(input: &str) -> Option<(&str, &str)> {
    let body = input.trim_start().strip_prefix('/')?;
    let end = body.find(char::is_whitespace).unwrap_or(body.len());
    let slug = &body[..end];
    // A slash makes it a path or a sentence, not a slug: /usr/bin is plain title text.
    (!slug.is_empty() && !slug.contains('/')).then(|| (slug, body[end..].trim_start()))
}

/// # Errors
///
/// Fails on a database error or a row whose body cannot be decoded.
pub async fn list(db: &Db) -> Result<Vec<IssueTemplate>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", name, slug, icon, color, body FROM issue_templates ORDER BY id"#
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(IssueTemplate {
                id: TemplateId(r.id),
                name: r.name,
                slug: r.slug,
                icon: r.icon,
                color: r.color,
                body: serde_json::from_str(&r.body).map_err(bad)?,
            })
        })
        .collect()
}

/// # Errors
///
/// `NotFound` when no template has the id; `Db` on other failures.
pub async fn get(db: &Db, id: TemplateId) -> Result<IssueTemplate> {
    list(db)
        .await?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| Error::NotFound(format!("template {}", id.0)))
}

/// The template `/slug` names. An unknown slug is `Invalid` and lists the known ones.
///
/// # Errors
///
/// `Invalid` for an unknown slug; `Db` on other failures.
pub async fn by_slug(db: &Db, slug: &str) -> Result<IssueTemplate> {
    let all = list(db).await?;
    if let Some(t) = all.iter().find(|t| t.slug == slug) {
        return Ok(t.clone());
    }
    let known: Vec<String> = all.iter().map(|t| format!("/{}", t.slug)).collect();
    let known = if known.is_empty() {
        "none".to_owned()
    } else {
        known.join(", ")
    };
    Err(Error::Invalid(format!(
        "no template /{slug} (known: {known})"
    )))
}

/// # Errors
///
/// `Invalid` for an empty name or slug, or a priority above 4; `Db` on other failures.
pub async fn create(db: &Db, mut input: TemplateInput, src: ChangeSource) -> Result<IssueTemplate> {
    validate(&mut input)?;
    let body = serde_json::to_string(&input.body).map_err(bad)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query!(
        "INSERT INTO issue_templates (name, slug, icon, color, body) VALUES (?, ?, ?, ?, ?)",
        input.name,
        input.slug,
        input.icon,
        input.color,
        body
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "issue_templates", id).await?;
    let created = json!(input.slug);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "template", &id.to_string(), &edit).await?;
    tx.commit().await?;
    get(db, TemplateId(id)).await
}

/// Replaces the template. Each changed field is recorded in the change history.
///
/// # Errors
///
/// As [`create`], and `NotFound` when no template has the id.
pub async fn update(
    db: &Db,
    id: TemplateId,
    mut input: TemplateInput,
    src: ChangeSource,
) -> Result<IssueTemplate> {
    validate(&mut input)?;
    let old = get(db, id).await?;
    let new_body = serde_json::to_value(&input.body).map_err(bad)?;
    let body_text = new_body.to_string();
    let pairs = [
        ("name", json!(old.name), json!(&input.name)),
        ("slug", json!(old.slug), json!(&input.slug)),
        ("icon", json!(old.icon), json!(&input.icon)),
        ("color", json!(old.color), json!(&input.color)),
        (
            "body",
            serde_json::to_value(&old.body).map_err(bad)?,
            new_body.clone(),
        ),
    ];
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "issue_templates", id.0).await?;
    let changed = sqlx::query!(
        "UPDATE issue_templates SET name = ?, slug = ?, icon = ?, color = ?, body = ? WHERE id = ?",
        input.name,
        input.slug,
        input.icon,
        input.color,
        body_text,
        id.0
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(Error::NotFound(format!("template {}", id.0)));
    }
    rows::changed(&mut tx, src, before).await?;
    for (field, o, n) in pairs {
        if o != n {
            let edit = Edit {
                field,
                old: Some(&o),
                new: Some(&n),
            };
            history::record(&mut tx, src, "template", &id.0.to_string(), &edit).await?;
        }
    }
    tx.commit().await?;
    get(db, id).await
}

/// # Errors
///
/// `NotFound` when no template has the id; `Db` on other failures.
pub async fn delete(db: &Db, id: TemplateId, src: ChangeSource) -> Result<()> {
    let old = get(db, id).await?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "issue_templates", id.0).await?;
    sqlx::query!("DELETE FROM issue_templates WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    let gone = json!(old.slug);
    let edit = Edit {
        field: "deleted",
        old: Some(&gone),
        new: None,
    };
    history::record(&mut tx, src, "template", &id.0.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(())
}

async fn exists(conn: &mut SqliteConnection, sql: &str, id: i64) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(sql)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .is_some())
}

/// Drops each reference that no longer resolves and says so in `skipped`.
async fn live_refs(
    conn: &mut SqliteConnection,
    mut f: TemplateFields,
    skipped: &mut Vec<String>,
) -> Result<TemplateFields> {
    if let Some(s) = f.status_id
        && !exists(conn, "SELECT 1 FROM statuses WHERE id = ?", s.0).await?
    {
        skipped.push(format!("status {} does not exist", s.0));
        f.status_id = None;
    }
    if let Some(p) = f.project_id
        && !exists(conn, "SELECT 1 FROM projects WHERE id = ?", p.0).await?
    {
        skipped.push(format!("project {} does not exist", p.0));
        f.project_id = None;
    }
    let mut labels = Vec::new();
    for l in f.label_ids {
        if exists(conn, "SELECT 1 FROM labels WHERE id = ?", l.0).await? {
            labels.push(l);
        } else {
            skipped.push(format!("label {} does not exist", l.0));
        }
    }
    f.label_ids = labels;
    Ok(f)
}

fn from_fields(title: String, f: TemplateFields) -> NewIssue {
    NewIssue {
        title,
        description: f.description,
        status_id: f.status_id,
        priority: f.priority,
        project_id: f.project_id,
        label_ids: Some(f.label_ids),
        estimate_minutes: f.estimate_minutes,
        properties: Some(f.properties),
        ..NewIssue::default()
    }
}

/// Creates the issue and its sub-issues in one transaction. The overrides win over the
/// template's fields; an override label list replaces the template's labels.
///
/// # Errors
///
/// `NotFound` when no template has the id; `Invalid` for a title that is empty after prefixing,
/// or a property value its definition rejects; `Db` on other failures.
pub async fn apply(
    db: &Db,
    id: TemplateId,
    overrides: NewIssue,
    src: ChangeSource,
) -> Result<AppliedTemplate> {
    let template = get(db, id).await?;
    let label = format!("Create from template {}", template.name);
    // Boxed: the transaction's state is large, and callers' futures must stay under clippy's limit.
    history::step(
        &label,
        Box::pin(apply_in(db, template.body, overrides, src)),
    )
    .await
}

/// The transaction behind [`apply`]; its changes join the caller's undo step.
async fn apply_in(
    db: &Db,
    body: TemplateBody,
    overrides: NewIssue,
    src: ChangeSource,
) -> Result<AppliedTemplate> {
    let mut skipped = Vec::new();
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let fields = live_refs(&mut tx, body.fields, &mut skipped).await?;
    let mut subs = Vec::new();
    for s in body.sub_issues {
        let f = live_refs(&mut tx, s.fields, &mut skipped).await?;
        subs.push(from_fields(s.title, f));
    }
    let title = if overrides.title.trim().is_empty() {
        body.title_prefix.trim_end().to_owned()
    } else {
        format!("{}{}", body.title_prefix, overrides.title)
    };
    let base = from_fields(title, fields);
    let mut props = base.properties.unwrap_or_default();
    props.extend(overrides.properties.unwrap_or_default());
    let main = NewIssue {
        title: base.title,
        description: overrides.description.or(base.description),
        status_id: overrides.status_id.or(base.status_id),
        priority: overrides.priority.or(base.priority),
        project_id: overrides.project_id.or(base.project_id),
        milestone_id: overrides.milestone_id,
        parent_id: overrides.parent_id,
        cycle_id: overrides.cycle_id,
        label_ids: overrides.label_ids.or(base.label_ids),
        start_date: overrides.start_date,
        due: overrides.due,
        estimate_minutes: overrides.estimate_minutes.or(base.estimate_minutes),
        properties: Some(props),
        created_at: overrides.created_at,
        completed_at: overrides.completed_at,
    };
    let issue = issues::create_in(&mut tx, main, src).await?;
    let mut sub_issues = Vec::new();
    for mut sub in subs {
        sub.parent_id = Some(issue.id);
        sub_issues.push(issues::create_in(&mut tx, sub, src).await?);
    }
    tx.commit().await?;
    Ok(AppliedTemplate {
        issue,
        sub_issues,
        skipped,
    })
}
