use serde_json::Value;
use sqlx::{Row, query::Query, sqlite::SqliteArguments, sqlite::SqliteRow};

use crate::{
    Db, Error, FieldRef, Filter, FixedField, Issue, Op, PropertyDef, PropertyKind, Result,
    SavedView, Sort, ViewDef, ViewId, issues,
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
        if key.contains('"') {
            return Err(Error::Invalid("invalid property key".into()));
        }
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
        FixedField::CreatedAt => ("issues.created_at", false),
        FixedField::UpdatedAt => ("issues.updated_at", false),
        FixedField::CompletedAt => ("issues.completed_at", false),
        FixedField::Archived => ("issues.archived_at", false),
        FixedField::Label => return Err(Error::Invalid("cannot sort by label".into())),
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

/// Compiles a view definition to SQL plus bind values. Property keys are checked against `defs` and
/// only ever travel as bind values.
///
/// # Errors
///
/// Fails with `Invalid` on an unknown property, a bad operator or value, or a filter nested too deeply.
pub fn compile(def: &ViewDef, defs: &[PropertyDef]) -> Result<(String, Vec<Value>)> {
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

fn view_from_row(r: &SqliteRow) -> Result<SavedView> {
    let def: String = r.try_get("definition")?;
    Ok(SavedView {
        id: ViewId(r.try_get("id")?),
        name: r.try_get("name")?,
        def: serde_json::from_str(&def).map_err(|e| Error::Invalid(format!("saved view: {e}")))?,
        position: r.try_get("position")?,
    })
}

/// # Errors
///
/// Fails on a database error or a stored definition that no longer parses.
pub async fn list(db: &Db) -> Result<Vec<SavedView>> {
    sqlx::query("SELECT id, name, definition, position FROM views ORDER BY position, id")
        .fetch_all(&db.pool)
        .await?
        .iter()
        .map(view_from_row)
        .collect()
}

/// Creates a view (at the end) when `id` is `None`, otherwise replaces name and definition.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn save(db: &Db, id: Option<ViewId>, name: &str, def: ViewDef) -> Result<SavedView> {
    let json = serde_json::to_string(&def).map_err(|e| Error::Invalid(e.to_string()))?;
    let id = if let Some(id) = id {
        let done = sqlx::query("UPDATE views SET name = ?, definition = ? WHERE id = ?")
            .bind(name)
            .bind(&json)
            .bind(id.0)
            .execute(&db.pool)
            .await?;
        if done.rows_affected() == 0 {
            return Err(Error::NotFound(format!("view {}", id.0)));
        }
        id.0
    } else {
        sqlx::query(
            "INSERT INTO views (name, definition, position)
             VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM views))",
        )
        .bind(name)
        .bind(&json)
        .execute(&db.pool)
        .await?
        .last_insert_rowid()
    };
    let row = sqlx::query("SELECT id, name, definition, position FROM views WHERE id = ?")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    view_from_row(&row)
}

/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn delete(db: &Db, id: ViewId) -> Result<()> {
    let done = sqlx::query("DELETE FROM views WHERE id = ?")
        .bind(id.0)
        .execute(&db.pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound(format!("view {}", id.0)));
    }
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
    for (position, id) in (0_i64..).zip(ids) {
        sqlx::query("UPDATE views SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
