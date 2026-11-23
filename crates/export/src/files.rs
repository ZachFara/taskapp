//! The export file formats. The writer and the rebuild share these types, so a file round-trips.

use std::fmt::Write as _;

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::FromRow;
use taskapp_core::{Error, Result};

/// Reads a regular file, never through a symlink: an export folder may come from a remote.
///
/// # Errors
///
/// `Invalid` when the path is a symlink or anything but a regular file.
pub fn read_plain(path: &std::path::Path) -> Result<String> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return Err(Error::Invalid(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    Ok(std::fs::read_to_string(path)?)
}

/// Whether `dir/sub` is a real directory. A missing one is `false`.
///
/// # Errors
///
/// `Invalid` when it exists and is a symlink or a file.
pub fn plain_dir(dir: &std::path::Path, sub: &str) -> Result<bool> {
    match std::fs::symlink_metadata(dir.join(sub)) {
        Ok(m) if m.is_dir() => Ok(true),
        Ok(_) => Err(Error::Invalid(format!(
            "{sub} in the export is not a real directory"
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub const SECTION_COMMENTS: &str = "\n## Comments\n";

pub fn invalid(e: impl std::fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}

/// Names that become file names come from user data, so only plain ASCII names pass.
///
/// # Errors
///
/// `Invalid` for an empty name or one with any other character.
pub fn safe_name(name: &str) -> Result<&str> {
    let plain = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if plain {
        Ok(name)
    } else {
        Err(Error::Invalid(format!("{name:?} cannot be a file name")))
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct StatusRow {
    pub name: String,
    pub category: String,
    pub position: i64,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// One entry of `project_statuses.yaml`.
#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ProjectStatusRow {
    pub name: String,
    pub category: String,
    pub position: i64,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct LabelRow {
    pub name: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// A custom relation kind; the built-in ones come with the database.
#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct RelationKindRow {
    pub id: String,
    pub name: String,
    pub forward: String,
    pub reverse: String,
    pub symmetric: bool,
    pub position: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct CycleRow {
    pub number: i64,
    pub name: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// `events/<id>.yaml`. Times are local wall-clock text; `issues` are keys.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EventFile {
    pub id: i64,
    pub title: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub all_day: bool,
    pub place: Option<String>,
    pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct PinRow {
    pub kind: String,
    pub target: String,
    pub title: Option<String>,
    pub position: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct MilestoneRow {
    pub name: String,
    pub target_date: Option<NaiveDate>,
}

/// `projects/<slug>.yaml`
#[derive(Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ProjectFile {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub doc_link: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub target_date: Option<NaiveDate>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// The project status name; an older export without one gets the first status.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: i64,
    #[sqlx(skip)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[sqlx(skip)]
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
    #[sqlx(skip)]
    #[serde(default)]
    pub milestones: Vec<MilestoneRow>,
}

/// One entry of `properties.yaml`. `kind` and `options` are the JSON the database stores.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyFile {
    pub key: String,
    pub name: String,
    pub kind: Value,
    pub options: Value,
    pub project: Option<String>,
    pub position: i64,
    #[serde(default = "applies_to_issues")]
    pub applies_to: String,
}

fn applies_to_issues() -> String {
    "issues".into()
}

/// One entry of `views.yaml`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewFile {
    pub name: String,
    pub position: i64,
    pub definition: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<i64>,
}

/// One entry of `view_templates.yaml`. The id is kept because views point at it.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewTemplateFile {
    pub id: i64,
    pub name: String,
    pub position: i64,
    pub definition: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// One entry of `issue_templates.yaml`; `body` is the JSON the database stores.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct IssueTemplateFile {
    pub name: String,
    pub slug: String,
    pub body: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// One entry of `import_log.yaml`: what was imported, so a re-import after a rebuild skips it.
/// The issue is named by number, never by database id.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportLogRow {
    pub external_id: String,
    pub source: String,
    pub issue: i64,
    pub imported_at: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub source_deleted: bool,
}

/// One planned work block of an issue, listed in its frontmatter.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkBlockRow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Frontmatter of `issues/<KEY>.md`. Every reference is by name or key, never by database id.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IssueFront {
    pub key: String,
    pub title: String,
    pub status: String,
    pub priority: i64,
    pub project: Option<String>,
    pub milestone: Option<String>,
    pub parent: Option<String>,
    pub cycle: Option<i64>,
    pub labels: Vec<String>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<DateTime<Utc>>,
    pub due_tz: Option<String>,
    pub estimate_minutes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate: Vec<String>,
    /// Every other kind, by kind id; blocks, related and duplicate keep their own keys so old exports read back.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub relations: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub work_blocks: Vec<WorkBlockRow>,
}

/// The frontmatter, the description, and the comments of one issue file.
pub type ParsedIssue = (IssueFront, String, Vec<CommentText>);

/// The created and updated times of a comment heading, and the text after its line.
type Heading<'a> = (DateTime<Utc>, DateTime<Utc>, &'a str);

#[derive(Debug, PartialEq)]
pub struct CommentText {
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Renders an issue file: frontmatter, the description as the body, then `## Comments` with one
/// `### <created> <updated>` block per comment.
///
/// Description lines `## Comments` get one extra leading backslash so they cannot start the section.
///
/// shortcut: a comment body containing a line `### <time> <time>` is read back wrongly. Escape it
/// if this ever happens in real notes.
///
/// # Errors
///
/// `Invalid` when the frontmatter cannot be encoded.
pub fn render_issue(
    front: &IssueFront,
    description: &str,
    comments: &[CommentText],
) -> Result<String> {
    let yaml = serde_yaml_ng::to_string(front).map_err(invalid)?;
    let mut out = format!("---\n{yaml}---\n{}", escape(description));
    if !comments.is_empty() {
        out.push_str(SECTION_COMMENTS);
    }
    for c in comments {
        let created = c.created_at.to_rfc3339_opts(SecondsFormat::AutoSi, true);
        let updated = c.updated_at.to_rfc3339_opts(SecondsFormat::AutoSi, true);
        write!(out, "\n### {created} {updated}\n\n{}\n", c.body).map_err(invalid)?;
    }
    Ok(out)
}

/// Reads an issue file written by `render_issue`.
///
/// # Errors
///
/// `Invalid` when the frontmatter or a comment heading does not parse.
pub fn parse_issue(text: &str) -> Result<ParsedIssue> {
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| invalid("issue file has no frontmatter"))?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| invalid("issue frontmatter is not closed"))?;
    let front: IssueFront = serde_yaml_ng::from_str(&rest[..=end]).map_err(invalid)?;
    let body = &rest[end + 5..];
    let (description, section) = match body.find(SECTION_COMMENTS) {
        Some(at) => (&body[..at], &body[at + SECTION_COMMENTS.len()..]),
        None => (body, ""),
    };
    Ok((front, unescape(description), parse_comments(section)?))
}

fn is_comments_line(line: &str) -> bool {
    line.trim_start_matches('\\') == "## Comments"
}

fn escape(text: &str) -> String {
    let lines: Vec<String> = text
        .split('\n')
        .map(|l| {
            if is_comments_line(l) {
                format!("\\{l}")
            } else {
                l.to_string()
            }
        })
        .collect();
    lines.join("\n")
}

fn unescape(text: &str) -> String {
    let lines: Vec<&str> = text
        .split('\n')
        .map(|l| {
            if is_comments_line(l) {
                l.strip_prefix('\\').unwrap_or(l)
            } else {
                l
            }
        })
        .collect();
    lines.join("\n")
}

fn parse_comments(section: &str) -> Result<Vec<CommentText>> {
    // A block runs from one heading to the newline that opens the next one (or the end).
    let padded = format!("\n{section}");
    let mut marks: Vec<usize> = padded
        .match_indices("\n### ")
        .map(|(i, _)| i)
        .filter(|&i| heading(&padded[i + 1..]).is_some())
        .collect();
    marks.push(padded.len());
    marks
        .windows(2)
        .map(|w| {
            let block = &padded[w[0] + 1..w[1]];
            let (created_at, updated_at, after) =
                heading(block).ok_or_else(|| invalid("comment heading"))?;
            let body = after
                .strip_prefix('\n')
                .and_then(|b| b.strip_suffix('\n'))
                .ok_or_else(|| invalid("comment body framing"))?;
            Ok(CommentText {
                body: body.to_string(),
                created_at,
                updated_at,
            })
        })
        .collect()
}

/// `### <created> <updated>\n...` at the start of `s`: the two times and the text after the heading line.
fn heading(s: &str) -> Option<Heading<'_>> {
    let line_end = s.find('\n')?;
    let mut parts = s[..line_end].strip_prefix("### ")?.split(' ');
    let created = DateTime::parse_from_rfc3339(parts.next()?).ok()?.to_utc();
    let updated = DateTime::parse_from_rfc3339(parts.next()?).ok()?.to_utc();
    parts
        .next()
        .is_none()
        .then(|| (created, updated, &s[line_end + 1..]))
}

/// The timestamp-free issue key for a number, e.g. `ABC-12`.
#[must_use]
pub fn key(prefix: &str, number: i64) -> String {
    format!("{prefix}-{number}")
}
