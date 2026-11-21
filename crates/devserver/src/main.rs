//! Development harness server: `commands::dispatch` over HTTP for the browser UI. No sync commands here.

mod seed;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Path as UrlPath, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::IntoResponse,
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

fn router(state: Arc<AppState>, port: u16) -> Router {
    // A predicate, not `exact`: the exact form sends the allow header to every origin.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|o, _| o == UI_ORIGIN))
        .allow_methods([axum::http::Method::POST])
        .allow_headers([header::CONTENT_TYPE]);
    // Only our own host names: a rebound DNS name still sends the attacker's Host.
    let hosts = Arc::new([format!("127.0.0.1:{port}"), format!("localhost:{port}")]);
    // Non-JSON bodies get 415 from the `Json` extractor, so a cross-site form post always needs a preflight.
    Router::new()
        .route("/rpc/{name}", post(rpc))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let hosts = hosts.clone();
            async move {
                let host = req
                    .headers()
                    .get(header::HOST)
                    .and_then(|h| h.to_str().ok());
                if host.is_some_and(|h| hosts.iter().any(|a| a == h)) {
                    next.run(req).await
                } else {
                    StatusCode::FORBIDDEN.into_response()
                }
            }
        }))
        .layer(cors)
        .with_state(state)
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
            let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
                .await
                .map_err(|e| e.to_string())?;
            let addr = listener.local_addr().map_err(|e| e.to_string())?;
            println!("listening on {addr}");
            let app = router(state, addr.port());
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
