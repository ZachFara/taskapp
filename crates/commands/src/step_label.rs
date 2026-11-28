//! The undo step's human label, built in one place from the command, its arguments and its result.
//! The UI shows it verbatim; the mock (ui/src/lib/api.mock.ts) builds the same text.

use serde_json::Value;

fn count(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

fn ids(args: &Value) -> usize {
    args["ids"].as_array().map_or(0, Vec::len)
}

/// `module_verb_rest` read as "Verb rest module", with the module made singular: `labels_update` is "Update label".
fn generic(name: &str) -> String {
    let mut parts = name.split('_');
    let plural = parts.next().unwrap_or_default();
    let module = match plural {
        p if p.ends_with("ies") => format!("{}y", &p[..p.len() - 3]),
        p if p.ends_with("ses") => p[..p.len() - 2].to_owned(),
        p => p.trim_end_matches('s').to_owned(),
    };
    let verb: Vec<&str> = parts.collect();
    let mut text = verb.join(" ");
    if !text.contains(&module) {
        text = format!("{text} {module}");
    }
    let mut chars = text.trim().chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

fn bulk(args: &Value) -> String {
    let n = ids(args);
    let patch = &args["patch"];
    if let Some(archive) = patch["archive"].as_bool() {
        let verb = if archive { "Archived" } else { "Restored" };
        return format!("{verb} {}", count(n, "issue"));
    }
    let set: Vec<&str> = patch["patch"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, v)| !v.is_null())
        .map(|(k, _)| k.as_str())
        .collect();
    match set.as_slice() {
        [one] => format!(
            "Set {} on {}",
            one.trim_end_matches("_id"),
            count(n, "issue")
        ),
        _ => format!("Edited {}", count(n, "issue")),
    }
}

/// The label for `name` called with `args`; `out` is the result once known, so a label that needs it
/// is only final after the command ran.
#[must_use]
pub fn step_label(name: &str, args: &Value, out: Option<&Value>) -> String {
    let key = || {
        out.and_then(|o| o["key"].as_str())
            .unwrap_or("issue")
            .to_owned()
    };
    match name {
        "issues_create" | "issues_create_from_quickadd" => format!("Created {}", key()),
        "issues_update" => format!("Edited {}", key()),
        "inbox_process" => format!("Processed {}", key()),
        "issues_bulk_update" => bulk(args),
        "issues_duplicate" => format!("Duplicated {}", count(ids(args), "issue")),
        "import_apply" => {
            let n = out.map_or(0, |o| o["created"].as_u64().unwrap_or(0));
            format!(
                "Imported {}",
                count(usize::try_from(n).unwrap_or(0), "issue")
            )
        }
        "pins_set" => {
            let verb = if args["pinned"].as_bool().unwrap_or(true) {
                "Pinned"
            } else {
                "Unpinned"
            };
            let n = args["targets"].as_array().map_or(0, Vec::len);
            format!("{verb} {}", count(n, "item"))
        }
        "projects_set_next" => if args["issue"].is_null() {
            "Cleared next action"
        } else {
            "Set next action"
        }
        .into(),
        "project_statuses_create" => "Created project status".into(),
        "project_statuses_update" => "Edited project status".into(),
        "project_statuses_reorder" => "Reordered project statuses".into(),
        "project_statuses_delete" => "Deleted project status".into(),
        "folders_place" => "Moved folder".into(),
        "folders_place_item" => format!("Moved {}", args["section"].as_str().unwrap_or("item")),
        "relation_kinds_create" => "Created relation kind".into(),
        "relation_kinds_update" => "Edited relation kind".into(),
        "relation_kinds_delete" => "Deleted relation kind".into(),
        "views_save" | "views_save_project" => {
            let view = args["name"].as_str().unwrap_or_default();
            if args["id"].is_null() {
                format!("Saved view {view}")
            } else {
                format!("Changed display of {view}")
            }
        }
        _ => generic(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn folder_moves_say_what_moved() {
        assert_eq!(
            step_label("folders_place", &json!({"id": 1}), None),
            "Moved folder"
        );
        let item = json!({"section": "view", "id": 3});
        assert_eq!(step_label("folders_place_item", &item, None), "Moved view");
        let pin = json!({"section": "pin", "id": 3});
        assert_eq!(step_label("folders_place_item", &pin, None), "Moved pin");
    }

    #[test]
    fn no_label_reads_like_a_command_name() {
        // Reads never make a step, so a parse fallback is not a label anyone sees.
        for n in crate::names().filter(|n| !n.contains("parse")) {
            let l = step_label(n, &json!({}), None);
            assert!(!l.contains('_') && !l.contains("quickadd"), "{n} => {l}");
        }
    }
}
