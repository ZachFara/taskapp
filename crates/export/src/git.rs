use std::path::Path;

use taskapp_core::Result;
use tokio::process::Command;

/// Stages everything, commits when something changed, then pushes when a remote is given.
/// A push is retried by the next call, because the commit stays in place when the push fails.
///
/// Returns whether a commit was made.
///
/// # Errors
///
/// Fails when git fails. A failed push leaves the commit in place.
pub async fn commit_and_push(dir: &Path, message: &str, remote: Option<&str>) -> Result<bool> {
    crate::write::refuse_empty(dir)?;
    let top = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await?;
    let top = String::from_utf8_lossy(&top.stdout);
    // Git resolves the repo upward, so a plain subfolder would stage and push the outer repo.
    if std::fs::canonicalize(top.trim()).ok() != std::fs::canonicalize(dir).ok() {
        return Err(taskapp_core::Error::Invalid(
            "export folder must be the root of its own git repo".into(),
        ));
    }
    git(dir, &["add", "-A"]).await?;
    let unchanged = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["diff", "--cached", "--quiet"])
        .status()
        .await?
        .success();
    if !unchanged {
        git(dir, &["commit", "-q", "-m", message]).await?;
    }
    if let Some(remote) = remote {
        git(dir, &["push", "-q", remote, "HEAD"]).await?;
    }
    Ok(!unchanged)
}

/// How the remote's branch relates to the local one, after a fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteState {
    /// Nothing the local branch has not seen (or the remote has no such branch yet).
    Current,
    /// The remote has commits this machine has not seen and the local branch has none of its own: fast-forward.
    Behind,
    /// Both sides have commits the other lacks. Needs a person.
    Diverged,
}

async fn succeeds(dir: &Path, args: &[&str]) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .await?;
    Ok(out.status.success())
}

/// Fetches the branch `HEAD` names from `remote` and compares it with the local one.
///
/// # Errors
///
/// Fails when git cannot reach the remote.
pub async fn remote_state(dir: &Path, remote: &str) -> Result<RemoteState> {
    let branch = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["symbolic-ref", "--short", "-q", "HEAD"])
        .output()
        .await?;
    let branch = String::from_utf8_lossy(&branch.stdout).trim().to_string();
    if branch.is_empty()
        || !succeeds(
            dir,
            &["ls-remote", "--exit-code", "--heads", remote, &branch],
        )
        .await?
    {
        return Ok(RemoteState::Current);
    }
    git(dir, &["fetch", "-q", remote, &branch]).await?;
    if !succeeds(dir, &["rev-parse", "--verify", "-q", "HEAD"]).await? {
        return Ok(RemoteState::Behind);
    }
    Ok(
        if succeeds(dir, &["merge-base", "--is-ancestor", "FETCH_HEAD", "HEAD"]).await? {
            RemoteState::Current
        } else if succeeds(dir, &["merge-base", "--is-ancestor", "HEAD", "FETCH_HEAD"]).await? {
            RemoteState::Behind
        } else {
            RemoteState::Diverged
        },
    )
}

/// Fast-forwards the local branch to what [`remote_state`] fetched last.
///
/// # Errors
///
/// Fails when the working tree blocks the fast-forward.
pub async fn fast_forward(dir: &Path) -> Result<()> {
    git(dir, &["merge", "-q", "--ff-only", "FETCH_HEAD"]).await
}

async fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .await?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(std::io::Error::other(format!("git {}: {}", args.join(" "), stderr.trim())).into())
}
