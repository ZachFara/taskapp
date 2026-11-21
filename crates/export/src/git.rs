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
