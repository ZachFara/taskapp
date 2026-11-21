use chrono::NaiveDate;

use crate::{ChangeSource, Db, Milestone, MilestoneId, Project, ProjectId, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(_db: &Db) -> Result<Vec<Project>> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(_db: &Db, _slug: &str, _name: &str, _src: ChangeSource) -> Result<Project> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(_db: &Db, _project: Project, _src: ChangeSource) -> Result<Project> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(_db: &Db, _id: ProjectId, _src: ChangeSource) -> Result<()> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list_milestones(_db: &Db, _project: ProjectId) -> Result<Vec<Milestone>> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_milestone(
    _db: &Db,
    _project: ProjectId,
    _name: &str,
    _target_date: Option<NaiveDate>,
    _src: ChangeSource,
) -> Result<Milestone> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update_milestone(
    _db: &Db,
    _milestone: Milestone,
    _src: ChangeSource,
) -> Result<Milestone> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_milestone(_db: &Db, _id: MilestoneId, _src: ChangeSource) -> Result<()> {
    todo!("T4")
}

/// `(completed, total)` issue counts, for project pages.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn progress(_db: &Db, _id: ProjectId) -> Result<(u32, u32)> {
    todo!("T4")
}
