//! Core: SQLite database and every domain operation.
//! One plain async function per UI command.

mod db;
mod error;
mod model;

pub mod backup;
pub mod blocks;
pub mod comments;
pub mod cycles;
pub mod history;
pub mod issues;
pub mod labels;
pub mod pins;
pub mod projects;
pub mod properties;
pub mod quickadd;
pub mod relations;
pub mod schedule;
pub mod search;
pub mod settings;
pub mod statuses;
pub mod templates;
pub mod views;

pub use db::Db;
pub use error::{Error, Result};
pub use model::*;
pub use templates::{Applied, IssueTemplate, TemplateInput};
