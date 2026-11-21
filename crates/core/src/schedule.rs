//! Pure scheduling logic; no database access.

use crate::{Bar, Issue, IssueId, Relation, Reschedule};

#[must_use]
pub fn bar_for(_issue: &Issue) -> Option<Bar> {
    todo!("T5")
}

#[must_use]
pub fn shift_dependents(
    _issues: &[Issue],
    _blocks: &[Relation],
    _moved: &Reschedule,
) -> Vec<Reschedule> {
    todo!("T5")
}

#[must_use]
pub fn critical_path(_issues: &[Issue], _blocks: &[Relation]) -> Vec<IssueId> {
    todo!("T5")
}
