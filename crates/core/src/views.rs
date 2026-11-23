use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, query::Query, sqlite::SqliteArguments, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, FieldRef, Filter, FixedField, Issue, IssueId, IssueStats, Op,
    PropertyDef, PropertyKind, Result, SavedView, Sort, ViewDef, ViewFolderId, ViewId,
    ViewTemplateId,
    history::{self, Edit},
    issues,
    palette::{self, Look},
    rows,
};

/// Filters come from saved JSON; a limit keeps a hostile one from overflowing the stack.
const MAX_DEPTH: usize = 32;

/// A column expression holding at most one `?`, for its bound value.
struct Col {
    sql: String,
    bind: Option<Value>,
    text: bool,
    multi: bool,
}

impl Col {
    fn fixed(sql: &str, text: bool) -> Col {
        Col {
            sql: sql.into(),
            bind: None,
            text,
            multi: false,
        }
    }
}

struct Ctx<'a> {
    defs: &'a [PropertyDef],
    binds: Vec<Value>,
}

fn scalar(v: &Value) -> Result<Value> {
    match v {
        Value::Array(_) | Value::Object(_) => Err(Error::Invalid("expected a single value".into())),
        _ => Ok(v.clone()),
    }
}

fn as_list(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| Error::Invalid("expected a list".into()))
}

fn marks(n: usize) -> String {
    vec!["?"; n].join(",")
}

impl Ctx<'_> {
    fn col(&self, f: &FieldRef) -> Result<Col> {
        let FieldRef::Property(key) = f else {
            let FieldRef::Fixed(x) = f else {
                unreachable!()
            };
            return fixed_col(*x);
        };
        let def = self
            .defs
            .iter()
            .find(|d| &d.key == key)
            .ok_or_else(|| Error::Invalid("unknown property".into()))?;
        Ok(Col {
            sql: "json_extract(issues.properties, ?)".into(),
            bind: Some(Value::String(format!("$.\"{key}\""))),
            text: !matches!(def.kind, PropertyKind::Number | PropertyKind::Checkbox),
            multi: def.kind == PropertyKind::MultiSelect,
        })
    }

    /// The column's SQL, with its bind queued. Call once per placeholder position.
    fn expr(&mut self, c: &Col) -> String {
        self.binds.extend(c.bind.clone());
        c.sql.clone()
    }

    fn filter(&mut self, f: &Filter, depth: usize) -> Result<String> {
        if depth > MAX_DEPTH {
            return Err(Error::Invalid("filter nested too deeply".into()));
        }
        match f {
            Filter::And(fs) => self.join(fs, " AND ", "1", depth),
            Filter::Or(fs) => self.join(fs, " OR ", "0", depth),
            Filter::Not(f) => Ok(format!("NOT ({})", self.filter(f, depth + 1)?)),
            Filter::Cond { field, op, value } => self.cond(field, *op, value),
        }
    }

    fn join(&mut self, fs: &[Filter], sep: &str, empty: &str, depth: usize) -> Result<String> {
        if fs.is_empty() {
            return Ok(empty.into());
        }
        let parts = fs
            .iter()
            .map(|f| self.filter(f, depth + 1))
            .collect::<Result<Vec<_>>>()?;
        Ok(format!("({})", parts.join(sep)))
    }

    fn cond(&mut self, field: &FieldRef, op: Op, value: &Value) -> Result<String> {
        match field {
            FieldRef::Fixed(FixedField::Label) => return self.label(op, value),
            FieldRef::Fixed(FixedField::Archived) => return archived(op, value),
            FieldRef::Fixed(FixedField::RelationKind) => return self.relation_kind(op, value),
            _ => {}
        }
        let c = self.col(field)?;
        match op {
            Op::IsEmpty | Op::IsNotEmpty => Ok(self.emptiness(&c, op == Op::IsEmpty)),
            Op::Contains => self.contains(&c, value),
            Op::In | Op::NotIn => self.membership(&c, op == Op::NotIn, value),
            _ => self.compare(&c, op, value),
        }
    }

    fn emptiness(&mut self, c: &Col, empty: bool) -> String {
        let e = self.expr(c);
        let e = if c.multi {
            format!("COALESCE(json_array_length({e}), 0) = 0")
        } else {
            format!("COALESCE({e}, '') = ''")
        };
        if empty { e } else { format!("NOT ({e})") }
    }

    fn contains(&mut self, c: &Col, value: &Value) -> Result<String> {
        let v = scalar(value)?;
        if c.multi {
            self.binds.extend(c.bind.clone());
            self.binds.push(v);
            return Ok(
                "EXISTS (SELECT 1 FROM json_each(issues.properties, ?) WHERE value = ?)".into(),
            );
        }
        if !c.text {
            return Err(Error::Invalid(
                "Contains needs a text or multi-select field".into(),
            ));
        }
        let e = self.expr(c);
        self.binds.push(v);
        Ok(format!("instr(lower({e}), lower(?)) > 0"))
    }

    fn membership(&mut self, c: &Col, negate: bool, value: &Value) -> Result<String> {
        let items = as_list(value)?;
        if items.is_empty() {
            return Ok(if negate { "1" } else { "0" }.into());
        }
        let e = self.expr(c);
        for i in items {
            self.binds.push(scalar(i)?);
        }
        let m = marks(items.len());
        // A missing value is not in the list, so NOT IN keeps it.
        Ok(if negate {
            format!("COALESCE({e} NOT IN ({m}), 1)")
        } else {
            format!("{e} IN ({m})")
        })
    }

    fn compare(&mut self, c: &Col, op: Op, value: &Value) -> Result<String> {
        let sym = match op {
            Op::Eq => "IS",
            Op::Neq => "IS NOT",
            Op::Lt => "<",
            Op::Lte => "<=",
            Op::Gt => ">",
            _ => ">=",
        };
        let e = self.expr(c);
        self.binds.push(scalar(value)?);
        Ok(format!("{e} {sym} ?"))
    }

    fn label(&mut self, op: Op, value: &Value) -> Result<String> {
        const ANY: &str = "EXISTS (SELECT 1 FROM issue_labels WHERE issue_id = issues.id";
        let (negate, items) = match op {
            Op::Eq | Op::Contains => (false, vec![scalar(value)?]),
            Op::Neq => (true, vec![scalar(value)?]),
            Op::In => (false, as_list(value)?.clone()),
            Op::NotIn => (true, as_list(value)?.clone()),
            Op::IsEmpty => return Ok(format!("NOT {ANY})")),
            Op::IsNotEmpty => return Ok(format!("{ANY})")),
            _ => return Err(Error::Invalid("unsupported operator for labels".into())),
        };
        if items.is_empty() {
            return Ok(if negate { "1" } else { "0" }.into());
        }
        let m = marks(items.len());
        for i in &items {
            self.binds.push(scalar(i)?);
        }
        let not = if negate { "NOT " } else { "" };
        Ok(format!("{not}{ANY} AND label_id IN ({m}))"))
    }

    fn relation_kind(&mut self, op: Op, value: &Value) -> Result<String> {
        const ANY: &str = "EXISTS (SELECT 1 FROM relations r WHERE (r.from_id = issues.id OR r.to_id = issues.id)";
        let (negate, items) = match op {
            Op::Eq | Op::Contains => (false, vec![scalar(value)?]),
            Op::Neq => (true, vec![scalar(value)?]),
            Op::In => (false, as_list(value)?.clone()),
            Op::NotIn => (true, as_list(value)?.clone()),
            Op::IsEmpty => return Ok(format!("NOT {ANY})")),
            Op::IsNotEmpty => return Ok(format!("{ANY})")),
            _ => {
                return Err(Error::Invalid(
                    "unsupported operator for relation kinds".into(),
                ));
            }
        };
        if items.is_empty() {
            return Ok(if negate { "1" } else { "0" }.into());
        }
        for i in &items {
            self.binds.push(scalar(i)?);
        }
        let not = if negate { "NOT " } else { "" };
        Ok(format!(
            "{not}{ANY} AND r.kind IN ({}))",
            marks(items.len())
        ))
    }

    fn order(&mut self, sorts: &[Sort]) -> Result<String> {
        let mut parts = vec![];
        for s in sorts {
            let c = self.col(&s.field)?;
            let (a, b) = (self.expr(&c), self.expr(&c));
            let dir = if s.descending { "DESC" } else { "ASC" };
            parts.push(format!("{a} IS NULL, {b} {dir}"));
        }
        parts.push("issues.id".into());
        Ok(parts.join(", "))
    }
}

fn fixed_col(x: FixedField) -> Result<Col> {
    let (sql, text) = match x {
        FixedField::Title => ("issues.title", true),
        FixedField::Status => ("issues.status_id", false),
        FixedField::StatusCategory => (
            "(SELECT category FROM statuses WHERE id = issues.status_id)",
            false,
        ),
        FixedField::Priority => ("issues.priority", false),
        FixedField::Project => ("issues.project_id", false),
        FixedField::Milestone => ("issues.milestone_id", false),
        FixedField::Cycle => ("issues.cycle_id", false),
        FixedField::Parent => ("issues.parent_id", false),
        FixedField::StartDate => ("issues.start_date", false),
        FixedField::DueDate => ("issues.due_date", false),
        FixedField::Estimate => ("issues.estimate_minutes", false),
        FixedField::Points => ("issues.points", false),
        FixedField::CreatedAt => ("issues.created_at", false),
        FixedField::UpdatedAt => ("issues.updated_at", false),
        FixedField::CompletedAt => ("issues.completed_at", false),
        FixedField::Archived => ("issues.archived_at", false),
        FixedField::Key => ("issues.number", false),
        FixedField::SubIssueProgress => (
            "(SELECT COUNT(*) FROM issues c WHERE c.parent_id = issues.id AND c.archived_at IS NULL)",
            false,
        ),
        FixedField::RelationCount => (
            "(SELECT COUNT(*) FROM relations r WHERE r.from_id = issues.id OR r.to_id = issues.id)",
            false,
        ),
        FixedField::CommentCount => (
            "(SELECT COUNT(*) FROM comments c WHERE c.issue_id = issues.id)",
            false,
        ),
        FixedField::Label => return Err(Error::Invalid("cannot sort by label".into())),
        FixedField::RelationKind => {
            return Err(Error::Invalid("cannot sort by relation kind".into()));
        }
    };
    Ok(Col::fixed(sql, text))
}

fn archived(op: Op, value: &Value) -> Result<String> {
    let want = match (op, value) {
        (Op::Eq, Value::Bool(b)) => *b,
        (Op::Neq, Value::Bool(b)) => !*b,
        _ => {
            return Err(Error::Invalid(
                "Archived takes Eq or Neq with a boolean".into(),
            ));
        }
    };
    Ok(format!(
        "issues.archived_at IS {}NULL",
        if want { "NOT " } else { "" }
    ))
}

fn strip_filter(f: Filter, gone: &dyn Fn(&str) -> bool) -> Option<Filter> {
    let list = |v: Vec<Filter>| {
        Some(
            v.into_iter()
                .filter_map(|f| strip_filter(f, gone))
                .collect::<Vec<_>>(),
        )
        .filter(|v| !v.is_empty())
    };
    match f {
        Filter::Cond {
            field: FieldRef::Property(ref k),
            ..
        } if gone(k) => None,
        Filter::And(v) => list(v).map(Filter::And),
        Filter::Or(v) => list(v).map(Filter::Or),
        Filter::Not(b) => strip_filter(*b, gone).map(|f| Filter::Not(Box::new(f))),
        other @ Filter::Cond { .. } => Some(other),
    }
}

/// Removes every reference to the properties `gone` names: filter conditions, sorts, columns, card fields, grouping.
pub(crate) fn scrub(def: &mut ViewDef, gone: &dyn Fn(&str) -> bool) {
    let hit = |f: &FieldRef| matches!(f, FieldRef::Property(k) if gone(k));
    def.filter = def.filter.take().and_then(|f| strip_filter(f, gone));
    def.sorts.retain(|s| !hit(&s.field));
    def.columns.retain(|c| !hit(c));
    def.card_fields.retain(|c| !hit(c));
    if def.group_by.as_ref().is_some_and(hit) {
        def.group_by = None;
    }
}

/// Compiles a view definition to SQL plus bind values. Property keys are checked against `defs` and
/// only ever travel as bind values.
///
/// # Errors
///
/// Fails with `Invalid` on a bad operator or value, or a filter nested too deeply.
pub fn compile(def: &ViewDef, defs: &[PropertyDef]) -> Result<(String, Vec<Value>)> {
    // A reference to a property that no longer exists is dropped, not an error: the view would stay broken.
    let mut def = def.clone();
    scrub(&mut def, &|k| !defs.iter().any(|d| d.key == k));
    let mut ctx = Ctx {
        defs,
        binds: vec![],
    };
    let mut wheres = vec![];
    if !def.include_archived {
        wheres.push("issues.archived_at IS NULL".to_string());
    }
    if let Some(f) = &def.filter {
        wheres.push(ctx.filter(f, 0)?);
    }
    let order = ctx.order(&def.sorts)?;
    let wheres = if wheres.is_empty() {
        "1".into()
    } else {
        wheres.join(" AND ")
    };
    Ok((
        format!(
            "SELECT {} FROM issues WHERE {wheres} ORDER BY {order}",
            issues::ISSUE_COLS
        ),
        ctx.binds,
    ))
}

fn bind_value<'q>(
    q: Query<'q, sqlx::Sqlite, SqliteArguments<'q>>,
    v: &'q Value,
) -> Query<'q, sqlx::Sqlite, SqliteArguments<'q>> {
    match v {
        Value::String(s) => q.bind(s.as_str()),
        Value::Bool(b) => q.bind(i64::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(i) => q.bind(i),
            None => q.bind(n.as_f64()),
        },
        _ => q.bind(None::<i64>),
    }
}

async fn property_defs(db: &Db) -> Result<Vec<PropertyDef>> {
    let rows =
        sqlx::query("SELECT key, name, kind, options, project_id, position FROM property_defs")
            .fetch_all(&db.pool)
            .await?;
    let parse = |e: serde_json::Error| Error::Invalid(format!("property definition: {e}"));
    rows.iter()
        .map(|r| {
            let kind: String = r.try_get("kind")?;
            let options: String = r.try_get("options")?;
            Ok(PropertyDef {
                key: r.try_get("key")?,
                name: r.try_get("name")?,
                kind: serde_json::from_str(&kind).map_err(parse)?,
                options: serde_json::from_str(&options).map_err(parse)?,
                project_id: r
                    .try_get::<Option<i64>, _>("project_id")?
                    .map(crate::ProjectId),
                position: r.try_get("position")?,
            })
        })
        .collect()
}

/// # Errors
///
/// Fails on a database error or an invalid view definition.
pub async fn query(db: &Db, def: &ViewDef) -> Result<Vec<Issue>> {
    let (sql, binds) = compile(def, &property_defs(db).await?)?;
    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = bind_value(q, b);
    }
    let rows = q.fetch_all(&db.pool).await?;
    issues::issues_from_rows(&mut *db.pool.acquire().await?, &rows).await
}

/// Sub-issue, relation and comment counts for the issues `def` shows, in the same order as [`query`].
///
/// # Errors
///
/// As [`query`].
pub async fn stats(db: &Db, def: &ViewDef) -> Result<Vec<IssueStats>> {
    let (sql, binds) = compile(def, &property_defs(db).await?)?;
    let sql = sql.replacen(
        &format!("SELECT {} FROM", issues::ISSUE_COLS),
        "SELECT issues.id,
           (SELECT COUNT(*) FROM issues c WHERE c.parent_id = issues.id AND c.archived_at IS NULL),
           (SELECT COUNT(*) FROM issues c JOIN statuses st ON st.id = c.status_id
              WHERE c.parent_id = issues.id AND c.archived_at IS NULL AND st.category = 'completed'),
           (SELECT COUNT(*) FROM relations r WHERE r.from_id = issues.id OR r.to_id = issues.id),
           (SELECT COUNT(*) FROM comments c WHERE c.issue_id = issues.id)
         FROM",
        1,
    );
    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = bind_value(q, b);
    }
    q.fetch_all(&db.pool)
        .await?
        .iter()
        .map(|r| {
            Ok(IssueStats {
                issue_id: IssueId(r.try_get(0)?),
                sub_total: r.try_get(1)?,
                sub_done: r.try_get(2)?,
                relations: r.try_get(3)?,
                comments: r.try_get(4)?,
            })
        })
        .collect()
}

fn view_from_row(r: &SqliteRow) -> Result<SavedView> {
    let def: String = r.try_get("definition")?;
    Ok(SavedView {
        id: ViewId(r.try_get("id")?),
        name: r.try_get("name")?,
        def: serde_json::from_str(&def).map_err(|e| Error::Invalid(format!("saved view: {e}")))?,
        position: r.try_get("position")?,
        icon: r.try_get("icon")?,
        color: r.try_get("color")?,
        template_id: r
            .try_get::<Option<i64>, _>("template_id")?
            .map(ViewTemplateId),
        folder_id: r.try_get::<Option<i64>, _>("folder_id")?.map(ViewFolderId),
    })
}

/// # Errors
///
/// Fails on a database error or a stored definition that no longer parses.
pub async fn list(db: &Db) -> Result<Vec<SavedView>> {
    sqlx::query(
        "SELECT id, name, definition, position, icon, color, template_id, folder_id FROM views ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?
    .iter()
    .map(view_from_row)
    .collect()
}

/// Creates a view (at the end) when `id` is `None`, otherwise replaces name, definition, icon and color.
/// On update an unset icon or color keeps the current one.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, `Invalid` for a bad icon or color, or on a database error.
pub async fn save(
    db: &Db,
    id: Option<ViewId>,
    name: &str,
    def: ViewDef,
    look: Look,
    src: ChangeSource,
) -> Result<SavedView> {
    let json = serde_json::to_string(&def).map_err(|e| Error::Invalid(e.to_string()))?;
    let Look { icon, color } = look;
    if let Some(icon) = &icon {
        palette::check_icon(icon)?;
    }
    let color = color
        .as_deref()
        .map(palette::key_for)
        .transpose()?
        .map(str::to_owned);
    let look = Look { icon, color };
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = match id {
        Some(id) => {
            update_view(&mut tx, src, id, name, &json, &look).await?;
            id.0
        }
        None => insert_view(&mut tx, src, name, &json, &look).await?,
    };
    let row = sqlx::query(
        "SELECT id, name, definition, position, icon, color, template_id, folder_id FROM views WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let view = view_from_row(&row)?;
    tx.commit().await?;
    Ok(view)
}

/// Records each changed field, then writes the row. An unset icon or color keeps the stored one.
async fn update_view(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    id: ViewId,
    name: &str,
    json: &str,
    look: &Look,
) -> Result<()> {
    let old = sqlx::query("SELECT name, icon, color FROM views WHERE id = ?")
        .bind(id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("view {}", id.0)))?;
    let before = rows::by_key(tx, "views", id.0).await?;
    let old_name: String = old.try_get("name")?;
    let old_icon: Option<String> = old.try_get("icon")?;
    let old_color: Option<String> = old.try_get("color")?;
    let icon = look.icon.as_deref().or(old_icon.as_deref());
    let color = look.color.as_deref().or(old_color.as_deref());
    let changes = [
        ("name", json!(old_name), json!(name)),
        ("icon", json!(old_icon), json!(icon)),
        ("color", json!(old_color), json!(color)),
    ];
    for (field, before, after) in changes {
        if before != after {
            let edit = Edit {
                field,
                old: Some(&before),
                new: Some(&after),
            };
            history::record(tx, src, "view", &id.0.to_string(), &edit).await?;
        }
    }
    sqlx::query("UPDATE views SET name = ?, definition = ?, icon = ?, color = ? WHERE id = ?")
        .bind(name)
        .bind(json)
        .bind(icon)
        .bind(color)
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(tx, src, before).await?;
    Ok(())
}

async fn insert_view(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    name: &str,
    json: &str,
    look: &Look,
) -> Result<i64> {
    let id = sqlx::query(
        "INSERT INTO views (name, definition, position, icon, color)
         VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM views), ?, ?)",
    )
    .bind(name)
    .bind(json)
    .bind(look.icon.as_deref())
    .bind(look.color.as_deref())
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(tx, src, "views", id).await?;
    let created = json!(name);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(tx, src, "view", &id.to_string(), &edit).await?;
    Ok(id)
}

/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn delete(db: &Db, id: ViewId) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "views", id.0).await?;
    let done = sqlx::query("DELETE FROM views WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound(format!("view {}", id.0)));
    }
    rows::changed(&mut tx, ChangeSource::Ui, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Sets every view's position to its index in `ids`.
///
/// # Errors
///
/// Fails with `Invalid` unless `ids` lists every view exactly once.
pub async fn reorder(db: &Db, ids: &[ViewId]) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut have: Vec<i64> = sqlx::query_scalar("SELECT id FROM views")
        .fetch_all(&mut *tx)
        .await?;
    let mut want: Vec<i64> = ids.iter().map(|id| id.0).collect();
    have.sort_unstable();
    want.sort_unstable();
    if want != have {
        return Err(Error::Invalid(
            "reorder must list every view exactly once".into(),
        ));
    }
    let before = rows::all(&mut tx, "views").await?;
    for (position, id) in (0_i64..).zip(ids) {
        sqlx::query("UPDATE views SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, ChangeSource::Ui, before).await?;
    tx.commit().await?;
    Ok(())
}
