use chrono::{Days, NaiveDate, Utc};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, query::Query, sqlite::SqliteArguments, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Deferred, Error, FieldRef, Filter, FixedField, Issue, IssueId, IssueStats,
    Op, ProjectId, ProjectRow, PropertyAppliesTo, PropertyDef, PropertyKind, Result, SavedView,
    Sort, ViewDef, ViewEntity, ViewFolderId, ViewId, ViewTemplateId,
    history::{self, Edit},
    insights, issues,
    palette::{self, Look},
    projects, rows,
};

/// Filters come from saved JSON; a limit keeps a hostile one from overflowing the stack.
const MAX_DEPTH: usize = 32;

/// A column expression holding at most one `?`, for its bound value.
struct Col {
    sql: String,
    bind: Option<Value>,
    text: bool,
    multi: bool,
    date: bool,
}

impl Col {
    fn fixed(sql: &str, text: bool) -> Col {
        Col {
            sql: sql.into(),
            bind: None,
            text,
            multi: false,
            date: false,
        }
    }
}

struct Ctx<'a> {
    defs: &'a [PropertyDef],
    binds: Vec<Value>,
    entity: ViewEntity,
    today: NaiveDate,
}

impl ViewEntity {
    /// The table a view of this entity lists.
    fn table(self) -> &'static str {
        match self {
            Self::Issue => "issues",
            Self::Project => "projects",
        }
    }
    fn word(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::Project => "project",
        }
    }
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
            let mut c = fixed_col(self.entity, *x)?;
            c.date = matches!(
                x,
                FixedField::StartDate
                    | FixedField::DueDate
                    | FixedField::HiddenUntil
                    | FixedField::TargetDate
                    | FixedField::CreatedAt
                    | FixedField::UpdatedAt
                    | FixedField::CompletedAt
            );
            return Ok(c);
        };
        let def = self
            .defs
            .iter()
            .find(|d| &d.key == key)
            .ok_or_else(|| Error::Invalid("unknown property".into()))?;
        Ok(Col {
            sql: format!("json_extract({}.properties, ?)", self.entity.table()),
            bind: Some(Value::String(format!("$.\"{key}\""))),
            text: !matches!(def.kind, PropertyKind::Number | PropertyKind::Checkbox),
            multi: def.kind == PropertyKind::MultiSelect,
            date: def.kind == PropertyKind::Date,
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
            FieldRef::Fixed(FixedField::Archived) => return archived(self.entity, op, value),
            FieldRef::Fixed(FixedField::RelationKind) if self.entity == ViewEntity::Issue => {
                return self.relation_kind(op, value);
            }
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
            return Ok(format!(
                "EXISTS (SELECT 1 FROM json_each({}.properties, ?) WHERE value = ?)",
                self.entity.table()
            ));
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
        let value = if c.date {
            relative_day(value, self.today)?
        } else {
            value.clone()
        };
        let e = self.expr(c);
        self.binds.push(scalar(&value)?);
        Ok(format!("{e} {sym} ?"))
    }

    fn label(&mut self, op: Op, value: &Value) -> Result<String> {
        let any = if self.entity == ViewEntity::Issue {
            "EXISTS (SELECT 1 FROM issue_labels WHERE issue_id = issues.id"
        } else {
            "EXISTS (SELECT 1 FROM project_labels WHERE project_id = projects.id"
        };
        let (negate, items) = match op {
            Op::Eq | Op::Contains => (false, vec![scalar(value)?]),
            Op::Neq => (true, vec![scalar(value)?]),
            Op::In => (false, as_list(value)?.clone()),
            Op::NotIn => (true, as_list(value)?.clone()),
            Op::IsEmpty => return Ok(format!("NOT {any})")),
            Op::IsNotEmpty => return Ok(format!("{any})")),
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
        Ok(format!("{not}{any} AND label_id IN ({m}))"))
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
        parts.push(format!("{}.id", self.entity.table()));
        Ok(parts.join(", "))
    }
}

/// `today`, `today+7` or `today-14` names a day counted from `today`; any other value is kept as it is.
fn relative_day(value: &Value, today: NaiveDate) -> Result<Value> {
    let Some(s) = value.as_str().filter(|s| s.starts_with("today")) else {
        return Ok(value.clone());
    };
    let bad = || Error::Invalid(format!("bad relative date {s}"));
    let rest = &s["today".len()..];
    let days: i64 = match rest.chars().next() {
        None => 0,
        Some('+' | '-') => rest.parse().map_err(|_| bad())?,
        Some(_) => return Err(bad()),
    };
    let day = if days < 0 {
        today.checked_sub_days(Days::new(days.unsigned_abs()))
    } else {
        today.checked_add_days(Days::new(days.unsigned_abs()))
    };
    day.map(|d| json!(d.to_string())).ok_or_else(bad)
}

fn fixed_col(entity: ViewEntity, x: FixedField) -> Result<Col> {
    if entity == ViewEntity::Project {
        return project_col(x);
    }
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
        FixedField::HiddenUntil => ("issues.hidden_until", false),
        FixedField::Depth => ("issues.depth", true),
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
        FixedField::ProjectStatus => (
            "(SELECT status_id FROM projects WHERE id = issues.project_id)",
            false,
        ),
        FixedField::Label => return Err(Error::Invalid("cannot sort by label".into())),
        FixedField::Checklist => return Err(Error::Invalid("cannot sort by checklist".into())),
        FixedField::RelationKind => {
            return Err(Error::Invalid("cannot sort by relation kind".into()));
        }
        FixedField::TargetDate
        | FixedField::Progress
        | FixedField::ProgressChart
        | FixedField::IssueCount => {
            return Err(Error::Invalid("a project field, not an issue field".into()));
        }
    };
    Ok(Col::fixed(sql, text))
}

fn project_col(x: FixedField) -> Result<Col> {
    let (sql, text) = match x {
        FixedField::Title => ("projects.name", true),
        FixedField::Status => ("projects.status_id", false),
        FixedField::StatusCategory => (
            "(SELECT category FROM project_statuses WHERE id = projects.status_id)",
            false,
        ),
        FixedField::Priority => ("projects.priority", false),
        FixedField::StartDate => ("projects.start_date", false),
        FixedField::TargetDate => ("projects.target_date", false),
        FixedField::Archived => ("projects.archived_at", false),
        FixedField::CreatedAt => ("projects.created_at", false),
        FixedField::UpdatedAt => ("projects.updated_at", false),
        FixedField::IssueCount => (
            "(SELECT COUNT(*) FROM issues i WHERE i.project_id = projects.id AND i.archived_at IS NULL)",
            false,
        ),
        // The sum is NULL for a project without issues; zero is what its bar shows.
        FixedField::Progress | FixedField::ProgressChart => (
            "(SELECT COALESCE(100.0 * SUM(s.category = 'completed') / NULLIF(COUNT(*), 0), 0) FROM issues i JOIN statuses s ON s.id = i.status_id WHERE i.project_id = projects.id AND i.archived_at IS NULL)",
            false,
        ),
        _ => return Err(Error::Invalid("not a project field".into())),
    };
    Ok(Col::fixed(sql, text))
}

fn archived(entity: ViewEntity, op: Op, value: &Value) -> Result<String> {
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
        "{}.archived_at IS {}NULL",
        entity.table(),
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
pub fn compile(
    def: &ViewDef,
    defs: &[PropertyDef],
    today: NaiveDate,
) -> Result<(String, Vec<Value>)> {
    compile_for(ViewEntity::Issue, def, defs, today)
}

/// As [`compile`] for either entity; a project view selects only the project ids.
fn compile_for(
    entity: ViewEntity,
    def: &ViewDef,
    defs: &[PropertyDef],
    today: NaiveDate,
) -> Result<(String, Vec<Value>)> {
    // A reference to a property that no longer exists is dropped, not an error: the view would stay broken.
    let mut def = def.clone();
    scrub(&mut def, &|k| !defs.iter().any(|d| d.key == k));
    let mut ctx = Ctx {
        defs,
        binds: vec![],
        entity,
        today,
    };
    let table = entity.table();
    let mut wheres = vec![];
    if !def.include_archived {
        wheres.push(format!("{table}.archived_at IS NULL"));
    }
    if def.deferred == Deferred::Hide && entity == ViewEntity::Issue {
        // A NaiveDate prints as YYYY-MM-DD only, so inlining it is safe and keeps the binds for the filter.
        wheres.push(format!(
            "(issues.hidden_until IS NULL OR issues.hidden_until <= '{today}')"
        ));
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
    let cols = match entity {
        ViewEntity::Issue => issues::ISSUE_COLS,
        ViewEntity::Project => "projects.id",
    };
    Ok((
        format!("SELECT {cols} FROM {table} WHERE {wheres} ORDER BY {order}"),
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

/// The property definitions a view of `entity` can use: issue views take every one, as before.
async fn property_defs(db: &Db, entity: ViewEntity) -> Result<Vec<PropertyDef>> {
    let rows = sqlx::query(
        "SELECT key, name, kind, options, project_id, position, applies_to FROM property_defs
         WHERE ? = 'issue' OR applies_to IN ('projects', 'both')",
    )
    .bind(entity.word())
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
                applies_to: match r.try_get::<String, _>("applies_to")?.as_str() {
                    "projects" => PropertyAppliesTo::Projects,
                    "both" => PropertyAppliesTo::Both,
                    _ => PropertyAppliesTo::Issues,
                },
            })
        })
        .collect()
}

/// # Errors
///
/// Fails on a database error or an invalid view definition.
pub async fn query(db: &Db, def: &ViewDef) -> Result<Vec<Issue>> {
    let (sql, binds) = compile(
        def,
        &property_defs(db, ViewEntity::Issue).await?,
        today(db).await?,
    )?;
    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = bind_value(q, b);
    }
    let rows = q.fetch_all(&db.pool).await?;
    issues::issues_from_rows(&mut *db.pool.acquire().await?, &rows).await
}

/// The projects a project view shows, in order, each with its issue counts.
///
/// # Errors
///
/// As [`query`].
pub async fn projects_query(db: &Db, def: &ViewDef) -> Result<Vec<ProjectRow>> {
    let (sql, binds) = compile_for(
        ViewEntity::Project,
        def,
        &property_defs(db, ViewEntity::Project).await?,
        today(db).await?,
    )?;
    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = bind_value(q, b);
    }
    let mut out = vec![];
    for r in q.fetch_all(&db.pool).await? {
        let id = ProjectId(r.try_get(0)?);
        let (issue_done, issue_total) = projects::progress(db, id).await?;
        let project = projects::find(&mut *db.pool.acquire().await?, id).await?;
        out.push(ProjectRow {
            project,
            issue_total,
            issue_done,
        });
    }
    Ok(out)
}

/// Sub-issue, relation and comment counts for the issues `def` shows, in the same order as [`query`].
///
/// # Errors
///
/// As [`query`].
pub async fn stats(db: &Db, def: &ViewDef) -> Result<Vec<IssueStats>> {
    let (sql, binds) = compile(
        def,
        &property_defs(db, ViewEntity::Issue).await?,
        today(db).await?,
    )?;
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

/// Today in the workspace time zone: what a relative date filter counts from.
async fn today(db: &Db) -> Result<NaiveDate> {
    Ok(insights::local(Utc::now(), insights::zone(db).await?))
}

fn view_from_row(r: &SqliteRow) -> Result<SavedView> {
    let def: String = r.try_get("definition")?;
    Ok(SavedView {
        id: ViewId(r.try_get("id")?),
        name: r.try_get("name")?,
        entity: serde_json::from_value(Value::String(r.try_get("entity")?))
            .map_err(|e| Error::Invalid(format!("saved view: {e}")))?,
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
        "SELECT id, name, entity, definition, position, icon, color, template_id, folder_id FROM views ORDER BY position, id",
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
    save_as(db, id, (name, ViewEntity::Issue), def, look, src).await
}

/// As [`save`], with what a new view lists. An existing view keeps its entity.
///
/// # Errors
///
/// As [`save`].
pub async fn save_as(
    db: &Db,
    id: Option<ViewId>,
    (name, entity): (&str, ViewEntity),
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
        None => insert_view(&mut tx, src, (name, entity), &json, &look).await?,
    };
    let row = sqlx::query(
        "SELECT id, name, entity, definition, position, icon, color, template_id, folder_id FROM views WHERE id = ?",
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
    (name, entity): (&str, ViewEntity),
    json: &str,
    look: &Look,
) -> Result<i64> {
    let id = sqlx::query(
        "INSERT INTO views (name, entity, definition, position, icon, color)
         VALUES (?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM views), ?, ?)",
    )
    .bind(name)
    .bind(entity.word())
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

/// Keeps the layout tab a view was last shown in. Picking a tab is looking, not editing, so this records no
/// change and makes no undo step; an undo of a later edit restores the row as it was, layout included.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn set_layout(db: &Db, id: ViewId, layout: crate::Layout) -> Result<()> {
    let word = serde_json::to_value(layout).map_err(|e| Error::Invalid(e.to_string()))?;
    let done = sqlx::query(
        "UPDATE views SET definition = json_set(definition, '$.layout', ?) WHERE id = ?",
    )
    .bind(word.as_str())
    .bind(id.0)
    .execute(&db.pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound(format!("view {}", id.0)));
    }
    Ok(())
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
