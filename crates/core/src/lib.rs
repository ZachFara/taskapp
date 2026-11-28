//! Core: SQLite database and every domain operation.
//! One plain async function per UI command.

mod command_registry;
mod db;
mod error;
mod model;

pub mod attachments;
pub mod backup;
pub mod blocks;
pub mod bulk;
pub mod checklist;
pub mod comments;
pub mod cycles;
pub mod decisions;
pub mod delete;
pub mod events;
pub mod focus;
pub mod forecast;
pub mod history;
pub mod import;
pub mod inbox;
pub mod insights;
pub mod issues;
pub mod labels;
pub mod links;
pub mod metrics;
pub mod next_actions;
pub mod palette;
pub mod pins;
pub mod project_statuses;
pub mod projects;
pub mod properties;
pub mod quickadd;
pub mod recur;
pub mod relation_kinds;
pub mod relations;
pub mod rows;
pub mod schedule;
pub mod search;
pub mod settings;
pub mod stale;
pub mod statuses;
pub mod templates;
pub mod undo;
pub mod view_folders;
pub mod view_templates;
pub mod views;
pub mod workspace;

pub use db::Db;
pub use error::{Error, Result};
pub use import::{Applied, ImportPlan};
pub use model::*;
pub use templates::{AppliedTemplate, IssueTemplate, TemplateInput};
pub use workspace::WorkspaceStatus;
