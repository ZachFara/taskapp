use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
};

use serde_json::{Value, json};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Server {
    child: Child,
    base: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

fn command(home: &Path, db: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_taskapp-devserver"));
    c.args(["serve", "--port", "0", "--db"])
        .arg(db)
        .env("HOME", home)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}

fn start(home: &Path, db: &Path) -> R<Server> {
    let mut child = command(home, db).spawn()?;
    let mut line = String::new();
    let stdout = child.stdout.take().ok_or("no stdout")?;
    BufReader::new(stdout).read_line(&mut line)?;
    let addr = line
        .trim()
        .strip_prefix("listening on ")
        .ok_or(line.clone())?;
    Ok(Server {
        base: format!("http://{addr}"),
        child,
    })
}

async fn rpc(s: &Server, cmd: &str, body: Value) -> R<Value> {
    Ok(reqwest::Client::new()
        .post(format!("{}/rpc/{cmd}", s.base))
        .json(&body)
        .send()
        .await?
        .json()
        .await?)
}

#[tokio::test]
async fn rpc_round_trip() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    // Settings are implemented in core already; issue commands still are stubs when this was written.
    let set = rpc(&s, "settings_set", json!({"key": "k", "value": "v"})).await?;
    assert_eq!(set["ok"], Value::Null, "{set}");
    let got = rpc(&s, "settings_get", json!({"key": "k"})).await?;
    assert_eq!(got["ok"], "v");
    Ok(())
}

#[tokio::test]
async fn unknown_command_returns_error_json() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    let r = rpc(&s, "no_such_command", json!({})).await?;
    assert_eq!(r["error"]["kind"], "NotFound", "{r}");
    assert!(r.get("ok").is_none());
    Ok(())
}

#[tokio::test]
async fn sync_commands_are_absent() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    let r = rpc(&s, "gcal_status", json!({})).await?;
    assert_eq!(r["error"]["kind"], "NotFound", "{r}");
    Ok(())
}

#[tokio::test]
async fn binds_localhost_only() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    assert!(s.base.starts_with("http://127.0.0.1:"), "{}", s.base);
    Ok(())
}

/// Like `output`, but a server that starts instead of refusing is killed and fails the test, not hangs it.
fn output_within_10s(mut c: Command) -> R<std::process::Output> {
    let mut child = c.spawn()?;
    for _ in 0..200 {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    child.kill()?;
    Err("the server kept running instead of refusing the database".into())
}

fn refused(home: &Path, db: &Path) -> R {
    let out = output_within_10s(command(home, db))?;
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("real app database"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}

fn fake_home() -> R<(tempfile::TempDir, std::path::PathBuf)> {
    let home = tempfile::tempdir()?;
    let app = home.path().join("Library/Application Support/taskapp");
    std::fs::create_dir_all(&app)?;
    let real = app.join("taskapp.db");
    std::fs::write(&real, "")?;
    Ok((home, real))
}

#[test]
fn refuses_real_app_db() -> R {
    let (home, real) = fake_home()?;
    refused(home.path(), &real)
}

#[test]
fn refuses_real_app_db_via_symlink() -> R {
    let (home, real) = fake_home()?;
    let link = home.path().join("link.db");
    std::os::unix::fs::symlink(&real, &link)?;
    refused(home.path(), &link)
}

#[test]
fn refuses_real_app_db_via_relative_path() -> R {
    let (home, _) = fake_home()?;
    let mut c = command(
        home.path(),
        Path::new("Library/Application Support/../Application Support/taskapp/taskapp.db"),
    );
    c.current_dir(home.path());
    let out = output_within_10s(c)?;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("real app database"));
    Ok(())
}

#[tokio::test]
async fn cors_allows_only_the_dev_origin() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    let post = |origin: &'static str| {
        reqwest::Client::new()
            .post(format!("{}/rpc/settings_get", s.base))
            .header("origin", origin)
            .json(&json!({"key": "k"}))
            .send()
    };
    let ok = post("http://localhost:5173").await?;
    assert_eq!(
        ok.headers()["access-control-allow-origin"],
        "http://localhost:5173"
    );
    let other = post("http://evil.example").await?;
    assert!(other.headers().get("access-control-allow-origin").is_none());
    Ok(())
}

#[tokio::test]
async fn rejects_foreign_host() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    let url = format!("{}/rpc/settings_get", s.base);
    let post = |host: String| {
        reqwest::Client::new()
            .post(&url)
            .header("host", host)
            .json(&json!({"key": "k"}))
            .send()
    };
    let number = s.base.rsplit(':').next().ok_or("no port")?;
    assert_eq!(post(format!("evil.example:{number}")).await?.status(), 403);
    assert_eq!(post("127.0.0.1".into()).await?.status(), 403);
    assert_eq!(post(format!("localhost:{number}")).await?.status(), 200);
    Ok(())
}

#[tokio::test]
async fn rejects_non_json_content_type() -> R {
    let dir = tempfile::tempdir()?;
    let s = start(dir.path(), &dir.path().join("t.db"))?;
    let res = reqwest::Client::new()
        .post(format!("{}/rpc/settings_set", s.base))
        .header("content-type", "text/plain")
        .body(r#"{"key":"k","value":"v"}"#)
        .send()
        .await?;
    assert_eq!(res.status(), 415);
    let got = rpc(&s, "settings_get", json!({"key": "k"})).await?;
    assert_eq!(
        got["ok"],
        Value::Null,
        "the command must not have run: {got}"
    );
    Ok(())
}

#[tokio::test]
async fn cors_origin_is_configurable() -> R {
    let dir = tempfile::tempdir()?;
    let mut c = command(dir.path(), &dir.path().join("t.db"));
    c.args(["--allow-origin", "http://localhost:5999"]);
    let mut child = c.spawn()?;
    let mut line = String::new();
    BufReader::new(child.stdout.take().ok_or("no stdout")?).read_line(&mut line)?;
    let s = Server {
        base: format!(
            "http://{}",
            line.trim()
                .strip_prefix("listening on ")
                .ok_or(line.clone())?
        ),
        child,
    };
    let post = |origin: &'static str| {
        reqwest::Client::new()
            .post(format!("{}/rpc/settings_get", s.base))
            .header("origin", origin)
            .json(&json!({"key": "k"}))
            .send()
    };
    let ok = post("http://localhost:5999").await?;
    assert_eq!(
        ok.headers()["access-control-allow-origin"],
        "http://localhost:5999"
    );
    assert!(
        post("http://localhost:5173")
            .await?
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
    Ok(())
}
