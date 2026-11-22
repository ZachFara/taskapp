//! Tool table and dispatch. Every tool is a thin call into core; errors come back as strings for the model.

use std::{path::PathBuf, sync::Arc};

use chrono::{DateTime, NaiveDate, Utc};
use rmcp::model::Tool;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use taskapp_core::{
    ChangeSource, Db, Issue, IssuePatch, NewIssue, Project, PropertyDef, PropertyKind, Relation,
    RelationKind, SelectOption, StatusCategory, ViewDef, backup, comments, cycles, history, issues,
    labels, pins, projects, properties, relations, search, statuses, undo, views,
};

type Res<T> = Result<T, String>;

/// Name, description, required arguments. Writes use the agent source, or the migration source when
/// the call passes `migration: true`.
type Row = (&'static str, &'static str, &'static [&'static str]);

const TOOLS: &[Row] = &[
    (
        "search_issues",
        "Full-text search over issues. Args: text, limit (default 20).",
        &["text"],
    ),
    (
        "query_view",
        "Run a view. Args: either def (a ViewDef) or view_id of a saved view.",
        &[],
    ),
    (
        "get_issue",
        "One issue by key with its relations, comments and children. Args: key.",
        &["key"],
    ),
    (
        "create_issue",
        "Create an issue. Args mirror NewIssue (title, status_id, priority, due, properties incl. linear_id, created_at, completed_at for migrated history); migration: true records the Migration source. Never reads or writes Linear.",
        &["title"],
    ),
    (
        "update_issue",
        "Change an issue by key. Args: key, patch (IssuePatch; null clears a nullable field).",
        &["key", "patch"],
    ),
    ("archive_issue", "Archive an issue. Args: key.", &["key"]),
    (
        "add_comment",
        "Comment on an issue. Args: key, body, created_at (optional, for migrated history).",
        &["key", "body"],
    ),
    (
        "add_relation",
        "Link two issues. Args: from_key, to_key, kind (Blocks, Related or Duplicate).",
        &["from_key", "to_key", "kind"],
    ),
    (
        "remove_relation",
        "Unlink two issues. Args: from_key, to_key, kind.",
        &["from_key", "to_key", "kind"],
    ),
    ("list_properties", "List custom property definitions.", &[]),
    (
        "create_property",
        "Create a custom property. Args: key, name, kind, options, project_id, position.",
        &["key", "name", "kind"],
    ),
    (
        "rename_property",
        "Rename a property, keeping its values. Args: key, new_name.",
        &["key", "new_name"],
    ),
    (
        "change_property_kind",
        "Change a property's type, converting values or refusing. Takes a database snapshot first. Args: key, kind, options.",
        &["key", "kind"],
    ),
    (
        "delete_property",
        "Delete a property and every issue's value for it. Takes a database snapshot first. Args: key.",
        &["key"],
    ),
    (
        "set_property",
        "Set or clear (null) one issue's property value. Args: key, property, value.",
        &["key", "property"],
    ),
    ("list_statuses", "List statuses.", &[]),
    (
        "create_status",
        "Create a status. Args: name, category, color.",
        &["name", "category", "color"],
    ),
    ("list_projects", "List projects.", &[]),
    (
        "create_project",
        "Create a project. Args: slug, name.",
        &["slug", "name"],
    ),
    (
        "create_milestone",
        "Add a milestone to a project. Args: project_id, name, target_date (optional).",
        &["project_id", "name"],
    ),
    ("list_cycles", "List cycles.", &[]),
    (
        "create_cycle",
        "Create a cycle. Args: start_date, end_date, name (optional).",
        &["start_date", "end_date"],
    ),
    ("list_labels", "List labels.", &[]),
    (
        "create_label",
        "Create a label. Args: name, color.",
        &["name", "color"],
    ),
    ("list_pins", "List pinned items.", &[]),
    (
        "pin",
        "Pin something. Args: kind (Issue, Project, View, Cycle, Note, Url), target, title (optional).",
        &["kind", "target"],
    ),
    ("unpin", "Remove a pin. Args: id.", &["id"]),
    (
        "update_project",
        "Change a project, including doc_link. Args: id plus any of name, description, doc_link, start_date, target_date (null clears).",
        &["id"],
    ),
    (
        "bulk_update",
        "Apply many patches. Args: items, a list of {key, patch}. More than 10 items takes a database snapshot first; an unknown key applies nothing.",
        &["items"],
    ),
    (
        "get_history",
        "Change history of an issue. Args: key.",
        &["key"],
    ),
    (
        "undo",
        "Undo the newest step (one tool call, however many issues it touched). Refuses when something changed since.",
        &[],
    ),
    ("redo", "Redo the step most recently undone.", &[]),
    (
        "undo_status",
        "Labels of the steps undo and redo would act on, or null.",
        &[],
    ),
];

pub fn definitions() -> Vec<Tool> {
    TOOLS
        .iter()
        .map(|(name, desc, required)| {
            let mut schema = Map::new();
            schema.insert("type".into(), json!("object"));
            schema.insert("required".into(), json!(required));
            Tool::new(*name, *desc, Arc::new(schema))
        })
        .collect()
}

pub struct Server {
    db: Db,
    backup_dir: PathBuf,
}

fn ce(e: &taskapp_core::Error) -> String {
    format!("{}: {e}", e.kind())
}

fn arg<T: DeserializeOwned>(a: &Value, k: &str) -> Res<T> {
    serde_json::from_value(a.get(k).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("invalid argument `{k}`: {e}"))
}

fn opt<T: DeserializeOwned>(a: &Value, k: &str) -> Res<Option<T>> {
    match a.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => arg(a, k).map(Some),
    }
}

fn ok<T: Serialize>(v: &T) -> Res<Value> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

fn src(a: &Value) -> ChangeSource {
    if a.get("migration") == Some(&Value::Bool(true)) {
        ChangeSource::Migration
    } else {
        ChangeSource::Agent
    }
}

impl Server {
    pub fn new(db: Db, backup_dir: PathBuf) -> Server {
        Server { db, backup_dir }
    }

    pub async fn call(&self, name: &str, args: Value) -> Res<Value> {
        let Some((_, _, required)) = TOOLS.iter().find(|t| t.0 == name) else {
            return Err(format!("unknown tool `{name}`"));
        };
        if let Some(missing) = required.iter().find(|k| args.get(**k).is_none()) {
            return Err(format!("missing required argument `{missing}`"));
        }
        // One tool call is one undoable step.
        let routed = history::step(name, async { Ok(self.route(name, &args).await) }).await;
        routed.unwrap_or_else(|e| Err(ce(&e)))
    }

    async fn route(&self, name: &str, args: &Value) -> Res<Value> {
        match name {
            "undo" => ok(&undo::undo(&self.db).await.map_err(|e| ce(&e))?),
            "redo" => ok(&undo::redo(&self.db).await.map_err(|e| ce(&e))?),
            "undo_status" => ok(&undo::undo_status(&self.db).await.map_err(|e| ce(&e))?),
            "search_issues" | "query_view" | "get_issue" | "create_issue" | "update_issue"
            | "archive_issue" | "add_comment" | "add_relation" | "remove_relation"
            | "bulk_update" | "get_history" => self.issue_tool(name, args).await,
            "list_properties"
            | "create_property"
            | "rename_property"
            | "change_property_kind"
            | "delete_property"
            | "set_property" => self.property_tool(name, args).await,
            _ => self.org_tool(name, args).await,
        }
    }

    async fn snapshot(&self, reason: &str) -> Res<()> {
        let done = backup::snapshot(&self.db, &self.backup_dir, reason).await;
        done.map(|_| ()).map_err(|e| ce(&e))
    }

    async fn issue_by_key(&self, a: &Value, k: &str) -> Res<Issue> {
        issues::get_by_key(&self.db, &arg::<String>(a, k)?)
            .await
            .map_err(|e| ce(&e))
    }

    async fn issue_tool(&self, name: &str, a: &Value) -> Res<Value> {
        let db = &self.db;
        let s = src(a);
        match name {
            "search_issues" => {
                let limit = opt::<u32>(a, "limit")?.unwrap_or(20);
                let found = search::search(db, &arg::<String>(a, "text")?, limit).await;
                ok(&found.map_err(|e| ce(&e))?)
            }
            "query_view" => self.query_view(a).await,
            "get_issue" => self.get_issue(a).await,
            "create_issue" => {
                let new: NewIssue = serde_json::from_value(a.clone()).map_err(|e| e.to_string())?;
                ok(&issues::create(db, new, s).await.map_err(|e| ce(&e))?)
            }
            "update_issue" => {
                let issue = self.issue_by_key(a, "key").await?;
                let patch: IssuePatch = arg(a, "patch")?;
                ok(&issues::update(db, issue.id, patch, s)
                    .await
                    .map_err(|e| ce(&e))?)
            }
            "archive_issue" => {
                let issue = self.issue_by_key(a, "key").await?;
                issues::archive(db, issue.id, s).await.map_err(|e| ce(&e))?;
                Ok(json!({ "archived": issue.key }))
            }
            "add_comment" => self.add_comment(a).await,
            "add_relation" | "remove_relation" => self.link(name == "add_relation", a).await,
            "bulk_update" => self.bulk_update(a).await,
            _ => self.history(a).await,
        }
    }

    async fn link(&self, add: bool, a: &Value) -> Res<Value> {
        let rel = self.relation(a).await?;
        let done = if add {
            relations::add(&self.db, rel, src(a)).await
        } else {
            relations::remove(&self.db, rel, src(a)).await
        };
        done.map_err(|e| ce(&e))?;
        Ok(json!({ "ok": true }))
    }

    async fn add_comment(&self, a: &Value) -> Res<Value> {
        let issue = self.issue_by_key(a, "key").await?;
        let at: Option<DateTime<Utc>> = opt(a, "created_at")?;
        let body = arg::<String>(a, "body")?;
        let made = comments::add_comment(&self.db, issue.id, &body, at, src(a)).await;
        ok(&made.map_err(|e| ce(&e))?)
    }

    async fn history(&self, a: &Value) -> Res<Value> {
        let issue = self.issue_by_key(a, "key").await?;
        let log = history::list_for(&self.db, "issue", &issue.id.0.to_string()).await;
        ok(&log.map_err(|e| ce(&e))?)
    }

    async fn query_view(&self, a: &Value) -> Res<Value> {
        let def: ViewDef = if let Some(id) = opt::<taskapp_core::ViewId>(a, "view_id")? {
            let all = views::list(&self.db).await.map_err(|e| ce(&e))?;
            all.into_iter()
                .find(|v| v.id == id)
                .ok_or("not_found: no saved view with that id")?
                .def
        } else {
            arg(a, "def").map_err(|_| "pass either `def` or `view_id`".to_string())?
        };
        ok(&views::query(&self.db, &def).await.map_err(|e| ce(&e))?)
    }

    async fn get_issue(&self, a: &Value) -> Res<Value> {
        let issue = self.issue_by_key(a, "key").await?;
        let db = &self.db;
        let rels = relations::for_issue(db, issue.id)
            .await
            .map_err(|e| ce(&e))?;
        let notes = comments::list(db, issue.id).await.map_err(|e| ce(&e))?;
        let kids = issues::children(db, issue.id).await.map_err(|e| ce(&e))?;
        Ok(json!({ "issue": issue, "relations": rels, "comments": notes, "children": kids }))
    }

    async fn relation(&self, a: &Value) -> Res<Relation> {
        let from = self.issue_by_key(a, "from_key").await?;
        let to = self.issue_by_key(a, "to_key").await?;
        let kind: RelationKind = arg(a, "kind")?;
        Ok(Relation {
            from: from.id,
            to: to.id,
            kind,
        })
    }

    async fn bulk_update(&self, a: &Value) -> Res<Value> {
        let items: Vec<Map<String, Value>> = arg(a, "items")?;
        if items.len() > 10 {
            self.snapshot("bulk_update").await?;
        }
        let mut plan = Vec::new();
        for item in items {
            let item = Value::Object(item);
            let issue = self.issue_by_key(&item, "key").await?;
            plan.push((issue.id, arg::<IssuePatch>(&item, "patch")?));
        }
        let n = plan.len();
        // One transaction: a failing item drops the rest, so nothing is half applied.
        let mut tx = self
            .db
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| ce(&e.into()))?;
        for (id, patch) in plan {
            issues::update_in(&mut tx, id, patch, src(a))
                .await
                .map_err(|e| ce(&e))?;
        }
        tx.commit().await.map_err(|e| ce(&e.into()))?;
        Ok(json!({ "updated": n }))
    }

    async fn property_def(&self, key: &str) -> Res<PropertyDef> {
        let defs = properties::list_defs(&self.db).await.map_err(|e| ce(&e))?;
        defs.into_iter()
            .find(|d| d.key == key)
            .ok_or_else(|| format!("not_found: no property `{key}`"))
    }

    async fn property_tool(&self, name: &str, a: &Value) -> Res<Value> {
        let db = &self.db;
        let s = src(a);
        match name {
            "list_properties" => ok(&properties::list_defs(db).await.map_err(|e| ce(&e))?),
            "create_property" => {
                let def: PropertyDef = serde_json::from_value(json!({
                    "key": a["key"], "name": a["name"], "kind": a["kind"],
                    "options": a.get("options").cloned().unwrap_or(json!([])),
                    "project_id": a.get("project_id").cloned().unwrap_or(Value::Null),
                    "position": a.get("position").cloned().unwrap_or(json!(0)),
                }))
                .map_err(|e| e.to_string())?;
                ok(&properties::create_def(db, def, s)
                    .await
                    .map_err(|e| ce(&e))?)
            }
            "rename_property" => {
                let (key, new) = (arg::<String>(a, "key")?, arg::<String>(a, "new_name")?);
                ok(&properties::rename_def(db, &key, &new, s)
                    .await
                    .map_err(|e| ce(&e))?)
            }
            "change_property_kind" => {
                let key: String = arg(a, "key")?;
                let kind: PropertyKind = arg(a, "kind")?;
                let options: Vec<SelectOption> = opt(a, "options")?.unwrap_or_default();
                // An unknown key must fail before the snapshot is taken.
                self.property_def(&key).await?;
                self.snapshot("change_property_kind").await?;
                let done = properties::change_kind(db, &key, kind, options, s).await;
                ok(&done.map_err(|e| ce(&e))?)
            }
            "delete_property" => {
                let key: String = arg(a, "key")?;
                self.property_def(&key).await?;
                self.snapshot("delete_property").await?;
                properties::delete_def(db, &key, s)
                    .await
                    .map_err(|e| ce(&e))?;
                Ok(json!({ "deleted": key }))
            }
            _ => {
                let issue = self.issue_by_key(a, "key").await?;
                let value: Option<Value> = opt(a, "value")?;
                let prop = arg::<String>(a, "property")?;
                let done = properties::set_value(db, issue.id, &prop, value, s).await;
                ok(&done.map_err(|e| ce(&e))?)
            }
        }
    }

    async fn org_tool(&self, name: &str, a: &Value) -> Res<Value> {
        let db = &self.db;
        let s = src(a);
        match name {
            "list_statuses" => ok(&statuses::list(db).await.map_err(|e| ce(&e))?),
            "create_status" => {
                let cat: StatusCategory = arg(a, "category")?;
                let (n, c) = (arg::<String>(a, "name")?, arg::<String>(a, "color")?);
                ok(&statuses::create(db, &n, cat, &c)
                    .await
                    .map_err(|e| ce(&e))?)
            }
            "list_projects" => ok(&projects::list(db).await.map_err(|e| ce(&e))?),
            "create_project" => {
                let (slug, n) = (arg::<String>(a, "slug")?, arg::<String>(a, "name")?);
                ok(&projects::create(db, &slug, &n, s)
                    .await
                    .map_err(|e| ce(&e))?)
            }
            "create_milestone" => {
                let id = arg(a, "project_id")?;
                let date: Option<NaiveDate> = opt(a, "target_date")?;
                let name = arg::<String>(a, "name")?;
                ok(&projects::create_milestone(db, id, &name, date, s)
                    .await
                    .map_err(|e| ce(&e))?)
            }
            "update_project" => self.update_project(a).await,
            "list_cycles" => ok(&cycles::list(db).await.map_err(|e| ce(&e))?),
            "create_cycle" => {
                let (start, end) = (arg(a, "start_date")?, arg(a, "end_date")?);
                let made = cycles::create(db, opt(a, "name")?, start, end, s).await;
                ok(&made.map_err(|e| ce(&e))?)
            }
            "list_labels" => ok(&labels::list(db).await.map_err(|e| ce(&e))?),
            "create_label" => {
                let (n, c) = (arg::<String>(a, "name")?, arg::<String>(a, "color")?);
                ok(&labels::create(db, &n, &c, s).await.map_err(|e| ce(&e))?)
            }
            "list_pins" => ok(&pins::list_resolved(db).await.map_err(|e| ce(&e))?),
            "pin" => {
                let (kind, target) = (arg(a, "kind")?, arg::<String>(a, "target")?);
                let made = pins::pin(db, kind, &target, opt(a, "title")?).await;
                ok(&made.map_err(|e| ce(&e))?)
            }
            _ => {
                pins::unpin(db, arg(a, "id")?).await.map_err(|e| ce(&e))?;
                Ok(json!({ "ok": true }))
            }
        }
    }

    /// Overlays the given fields on the stored project, so null clears a nullable field.
    async fn update_project(&self, a: &Value) -> Res<Value> {
        let id: taskapp_core::ProjectId = arg(a, "id")?;
        let all = projects::list(&self.db).await.map_err(|e| ce(&e))?;
        let current = all
            .into_iter()
            .find(|p| p.id == id)
            .ok_or("not_found: no such project")?;
        let mut merged = ok(&current)?;
        for k in [
            "name",
            "description",
            "doc_link",
            "start_date",
            "target_date",
        ] {
            if let Some(v) = a.get(k) {
                merged[k] = v.clone();
            }
        }
        let project: Project = serde_json::from_value(merged).map_err(|e| e.to_string())?;
        ok(&projects::update(&self.db, project, src(a))
            .await
            .map_err(|e| ce(&e))?)
    }
}
