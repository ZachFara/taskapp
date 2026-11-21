use chrono::NaiveDate;

use crate::{ChangeSource, Db, Issue, ParsedQuickAdd, Result};

/// Pure: parses quick-add text relative to `today` in IANA zone `tz`.
#[must_use]
pub fn parse(_input: &str, _today: NaiveDate, _tz: &str) -> ParsedQuickAdd {
    todo!("T7")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_from(
    _db: &Db,
    _input: &str,
    _today: NaiveDate,
    _tz: &str,
    _src: ChangeSource,
) -> Result<Issue> {
    todo!("T7")
}
