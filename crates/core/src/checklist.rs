//! Markdown task lines (`- [ ] item`) in an issue description. The description is the source of truth; this only counts.
//! Counting matches the UI's checklist.ts, so a box in the app and the count in the CLI agree.

/// The (done, total) task items outside fenced code blocks.
#[must_use]
pub fn counts(description: &str) -> (usize, usize) {
    let mut fenced = false;
    let (mut done, mut total) = (0, 0);
    for line in description.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if let Some(checked) = task_box(t).filter(|_| !fenced) {
            total += 1;
            done += usize::from(checked);
        }
    }
    (done, total)
}

/// Some(ticked) when the line is a task item: a bullet, a space, then `[ ]`, `[x]` or `[X]` ending the box.
fn task_box(line: &str) -> Option<bool> {
    let rest = line.strip_prefix(['-', '*', '+'])?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let inner = rest.trim_start().strip_prefix('[')?;
    let mark = inner
        .chars()
        .next()
        .filter(|c| matches!(c, ' ' | 'x' | 'X'))?;
    let end = inner[mark.len_utf8()..].strip_prefix(']')?;
    (end.is_empty() || end.starts_with(' ')).then_some(mark != ' ')
}

/// The description with every ticked task box unticked (a recurring issue's next copy); code fences are left alone.
#[must_use]
pub fn reset(description: &str) -> String {
    let mut fenced = false;
    let lines: Vec<String> = description
        .lines()
        .map(|line| {
            let t = line.trim_start();
            if t.starts_with("```") || t.starts_with("~~~") {
                fenced = !fenced;
            } else if !fenced && task_box(t) == Some(true) {
                let at = line.find('[').unwrap_or(0);
                return format!("{}[ {}", &line[..at], &line[at + 2..]);
            }
            line.to_string()
        })
        .collect();
    lines.join("\n")
}
