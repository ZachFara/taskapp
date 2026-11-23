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

/// Whether `source` names a remote (`https://`, `ssh://` or `git@host:path`) rather than a folder.
#[must_use]
pub fn is_url(source: &str) -> bool {
    source.contains("://")
        || source
            .split_once(':')
            .is_some_and(|(user_host, _)| user_host.contains('@') && !user_host.contains('/'))
}

/// Clones `url` into `dest` with the machine's own git and refuses a tree with links in it.
/// Only https and ssh URLs pass; the URL goes to git as an argument after `--`, never through a shell.
///
/// # Errors
///
/// `Invalid` for another scheme, a leading `-` or a tree with a symlink; git's own message when the clone fails.
pub async fn clone(url: &str, dest: &Path) -> Result<()> {
    clone_allowing(url, dest, "never").await
}

/// [`clone`] with a choice for the `file` protocol; the tests clone a local bare repo with `always`.
pub(crate) async fn clone_allowing(url: &str, dest: &Path, file: &str) -> Result<()> {
    let ok = match url.split_once("://") {
        Some((scheme, rest)) => {
            (matches!(scheme, "https" | "ssh") || (scheme == "file" && file == "always"))
                && !rest.is_empty()
        }
        None => is_url(url),
    };
    if !ok || url.starts_with('-') {
        return Err(taskapp_core::Error::Invalid(
            "use an https:// or ssh:// Git URL, or git@host:path".into(),
        ));
    }
    let out = Command::new("git")
        .args([
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
        ])
        .args(["-c", "protocol.ssh.allow=always"])
        .args([
            "-c",
            &format!("protocol.file.allow={file}"),
            "clone",
            "-q",
            "--",
        ])
        .arg(url)
        .arg(dest)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(std::io::Error::other(format!("git clone: {}", stderr.trim())).into());
    }
    reject_links_at(dest, "HEAD").await
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

/// Refuses a fetched tree that holds a symlink or a submodule: the export would read and write through it.
///
/// # Errors
///
/// `Invalid` naming the first such path.
pub async fn reject_links(dir: &Path) -> Result<()> {
    reject_links_at(dir, "FETCH_HEAD").await
}

/// [`reject_links`] for any revision, such as the `HEAD` of a fresh clone.
///
/// # Errors
///
/// As [`reject_links`].
pub async fn reject_links_at(dir: &Path, rev: &str) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-tree", "-r", rev, "--"])
        .output()
        .await?;
    if !out.status.success() {
        return Err(std::io::Error::other("git ls-tree failed").into());
    }
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let (meta, path) = line.split_once('\t').unwrap_or((line, ""));
        if meta.starts_with("120000") || meta.starts_with("160000") {
            return Err(taskapp_core::Error::Invalid(format!(
                "the export has a symlink or submodule at {path:?}; nothing was loaded"
            )));
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_told_from_folders() {
        for u in [
            "https://h/x.git",
            "ssh://git@h/x",
            "git@github.com:me/x.git",
        ] {
            assert!(is_url(u), "{u}");
        }
        for f in ["/tmp/x", "~/Documents/export", "relative/dir", "C:/x"] {
            assert!(!is_url(f), "{f}");
        }
    }

    #[tokio::test]
    async fn other_schemes_and_option_lookalikes_are_refused() {
        let dest = std::env::temp_dir().join("never-created-by-clone-test");
        for u in [
            "file:///tmp/x",
            "ext::sh -c id",
            "--upload-pack=id",
            "ftp://h/x",
            "http://h/x",
        ] {
            let err = clone(u, &dest).await;
            assert!(
                matches!(err, Err(taskapp_core::Error::Invalid(_))),
                "{u}: {err:?}"
            );
        }
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn clone_takes_a_local_bare_repo_and_refuses_one_with_a_symlink() {
        let root = tempfile::tempdir().expect("tmp");
        let run = |dir: &Path, args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
                .args(args)
                .status()
                .expect("git")
                .success();
            assert!(ok, "git {args:?}");
        };
        let work = root.path().join("work");
        let bare = root.path().join("bare.git");
        std::fs::create_dir(&work).expect("mkdir");
        run(&work, &["init", "-q"]);
        std::fs::write(work.join("a.yaml"), "[]").expect("write");
        run(&work, &["add", "-A"]);
        run(&work, &["commit", "-q", "-m", "one"]);
        run(root.path(), &["clone", "-q", "--bare", "work", "bare.git"]);
        let url = format!("file://{}", bare.display());
        let ok = root.path().join("ok");
        clone_allowing(&url, &ok, "always").await.expect("clone");
        assert!(ok.join("a.yaml").is_file());

        std::os::unix::fs::symlink("/etc/hosts", work.join("link")).expect("link");
        run(&work, &["add", "-A"]);
        run(&work, &["commit", "-q", "-m", "two"]);
        run(
            &work,
            &["push", "-q", bare.to_str().expect("utf-8"), "HEAD"],
        );
        let err = clone_allowing(&url, &root.path().join("bad"), "always").await;
        assert!(
            matches!(err, Err(taskapp_core::Error::Invalid(_))),
            "{err:?}"
        );
    }
}
