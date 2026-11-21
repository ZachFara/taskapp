//! Development harness server: `commands::dispatch` over HTTP for the browser UI. No sync commands here.

mod seed;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Path as UrlPath, State},
    routing::post,
};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use taskapp_commands::{AppState, dispatch};
use taskapp_core::Db;
use tower_http::cors::{AllowOrigin, CorsLayer};

const UI_ORIGIN: &str = "http://localhost:5173";

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Serve `POST /rpc/<command>` on 127.0.0.1.
    Serve {
        #[arg(long)]
        db: PathBuf,
        #[arg(long, default_value_t = 7878)]
        port: u16,
    },
    /// Fill a database with the fixtures the UI mock uses.
    Seed {
        #[arg(long)]
        db: PathBuf,
    },
}

/// Absolute path with symlinks resolved as far as the path exists; the rest is appended as written.
fn resolve(p: &Path) -> std::io::Result<PathBuf> {
    let abs = std::path::absolute(p)?;
    let mut rest = PathBuf::new();
    let mut head = abs.as_path();
    loop {
        if let Ok(real) = head.canonicalize() {
            return Ok(real.join(rest));
        }
        let (Some(parent), Some(name)) = (head.parent(), head.file_name()) else {
            return Ok(abs);
        };
        rest = Path::new(name).join(rest);
        head = parent;
    }
}

/// The app's data directory holds the db and its WAL files; nothing under it may be opened.
fn refuse_real_db(db: &Path) -> Result<(), String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    let app = resolve(&Path::new(&home).join("Library/Application Support/taskapp"))
        .map_err(|e| e.to_string())?;
    if resolve(db).map_err(|e| e.to_string())?.starts_with(&app) {
        return Err(format!("{} is the real app database", db.display()));
    }
    Ok(())
}

async fn rpc(
    State(state): State<Arc<AppState>>,
    UrlPath(name): UrlPath<String>,
    Json(args): Json<Value>,
) -> Json<Value> {
    Json(match dispatch(&state, &name, args).await {
        Ok(v) => json!({ "ok": v }),
        Err(e) => json!({ "error": e }),
    })
}

async fn run(cmd: Cmd) -> Result<(), String> {
    let (Cmd::Serve { db, .. } | Cmd::Seed { db }) = &cmd;
    refuse_real_db(db)?;
    let db = Db::open(db).await.map_err(|e| e.to_string())?;
    match cmd {
        Cmd::Seed { .. } => seed::run(&db).await.map_err(|e| e.to_string()),
        Cmd::Serve { port, .. } => {
            let state = Arc::new(AppState {
                db,
                helper_path: PathBuf::new(),
                tz: std::env::var("TZ").unwrap_or_else(|_| "UTC".into()),
            });
            // A predicate, not `exact`: the exact form sends the allow header to every origin.
            let cors = CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|o, _| o == UI_ORIGIN))
                .allow_methods([axum::http::Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE]);
            let app = Router::new()
                .route("/rpc/{name}", post(rpc))
                .layer(cors)
                .with_state(state);
            let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "listening on {}",
                listener.local_addr().map_err(|e| e.to_string())?
            );
            axum::serve(listener, app).await.map_err(|e| e.to_string())
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run(Cli::parse().cmd).await {
        eprintln!("taskapp-devserver: {e}");
        std::process::exit(1);
    }
}
