//! Google Calendar REST calls. Every function takes the endpoints in a `Google` so tests can point at a mock.

use chrono::{DateTime, Duration, Utc};
use reqwest::{Method, RequestBuilder, StatusCode, Url};
use serde_json::{Value, json};
use taskapp_core::{Db, Error, Result, SyncState, SyncStatus, settings};

use crate::{
    auth,
    planner::{DesiredEvent, EventTiming},
};

pub const DISCONNECTED: &str = "Google Calendar disconnected. Reconnect in Settings.";

#[derive(Clone)]
pub struct Google {
    pub api: String,
    pub auth_url: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Google {
    /// # Errors
    ///
    /// Fails when the build had no `GOOGLE_CLIENT_ID`.
    pub fn production() -> Result<Google> {
        let client_id = option_env!("GOOGLE_CLIENT_ID")
            .ok_or_else(|| Error::Invalid("this build has no GOOGLE_CLIENT_ID".into()))?;
        Ok(Google {
            api: "https://www.googleapis.com/calendar/v3".into(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            client_id: client_id.into(),
            // Google issues a secret for Desktop clients and still wants it; it is not confidential.
            client_secret: option_env!("GOOGLE_CLIENT_SECRET")
                .unwrap_or_default()
                .into(),
        })
    }
}

/// An event the calendar currently holds that carries the app's marker.
pub struct Remote {
    pub id: String,
    pub etag: String,
    pub managed_key: String,
}

fn bad(e: impl std::fmt::Display) -> Error {
    Error::Invalid(format!("Google Calendar request failed: {e}"))
}

fn net(e: reqwest::Error) -> Error {
    bad(e.without_url())
}

fn url(g: &Google, segments: &[&str]) -> Result<Url> {
    let mut u = Url::parse(&g.api).map_err(bad)?;
    u.path_segments_mut()
        .map_err(|()| bad("bad api url"))?
        .extend(segments);
    Ok(u)
}

async fn send(req: RequestBuilder) -> Result<(StatusCode, Value)> {
    let resp = req.send().await.map_err(net)?;
    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(Error::Invalid(DISCONNECTED.into()));
    }
    let body = resp.json().await.unwrap_or(Value::Null);
    Ok((status, body))
}

fn ok(status: StatusCode, body: Value) -> Result<Value> {
    if status.is_success() {
        Ok(body)
    } else {
        Err(bad(format!("status {status}")))
    }
}

/// Trades the refresh token for an access token. A rejected refresh token means the grant is gone.
///
/// # Errors
///
/// Fails with the disconnected message when Google rejects the refresh token.
pub async fn access_token(g: &Google, refresh_token: &str) -> Result<String> {
    let resp = reqwest::Client::new()
        .post(&g.token_url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &g.client_id),
            ("client_secret", &g.client_secret),
        ])
        .send()
        .await
        .map_err(net)?;
    let status = resp.status();
    if status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED {
        return Err(Error::Invalid(DISCONNECTED.into()));
    }
    let body = ok(status, resp.json().await.map_err(net)?)?;
    body["access_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| bad("no access token in the response"))
}

/// Records `result`'s disconnected failure where the UI shows it, then passes the result on.
pub(crate) async fn flag<T>(db: &Db, result: Result<T>) -> Result<T> {
    if matches!(&result, Err(Error::Invalid(m)) if m == DISCONNECTED) {
        let state = SyncState::Error {
            at: Utc::now(),
            message: DISCONNECTED.into(),
        };
        set_state(db, state).await?;
    }
    result
}

pub(crate) async fn set_state(db: &Db, state: SyncState) -> Result<()> {
    let mut status = settings::get::<SyncStatus>(db, "sync_status")
        .await?
        .unwrap_or(SyncStatus {
            reminders: SyncState::Off,
            gcal: SyncState::Off,
            export: SyncState::Off,
            backup: SyncState::Off,
        });
    status.gcal = state;
    settings::set(db, "sync_status", &status).await
}

/// `(id, name)` of each calendar the account can write to.
///
/// # Errors
///
/// Fails when not connected or the request fails.
pub async fn list_calendars(db: &Db) -> Result<Vec<(String, String)>> {
    let g = Google::production()?;
    let r = list_calendars_with(&g, &auth::load_token()?).await;
    flag(db, r).await
}

/// `list_calendars` against explicit endpoints and refresh token.
///
/// # Errors
///
/// As `list_calendars`.
pub async fn list_calendars_with(g: &Google, refresh_token: &str) -> Result<Vec<(String, String)>> {
    let token = access_token(g, refresh_token).await?;
    let req = reqwest::Client::new()
        .get(url(g, &["users", "me", "calendarList"])?)
        .query(&[("minAccessRole", "writer")])
        .bearer_auth(token);
    let (status, body) = send(req).await?;
    let body = ok(status, body)?;
    let text = |v: &Value, k: &str| v[k].as_str().unwrap_or_default().to_owned();
    Ok(body["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| (text(c, "id"), text(c, "summary")))
        .collect())
}

/// Every live event in `cal` that carries the app's marker.
pub(crate) async fn list_managed(g: &Google, token: &str, cal: &str) -> Result<Vec<Remote>> {
    let mut found = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut req = reqwest::Client::new()
            .get(url(g, &["calendars", cal, "events"])?)
            .query(&[
                ("privateExtendedProperty", "taskapp=1"),
                ("maxResults", "2500"),
            ])
            .bearer_auth(token);
        if let Some(p) = &page {
            req = req.query(&[("pageToken", p)]);
        }
        let (status, body) = send(req).await?;
        let body = ok(status, body)?;
        for e in body["items"].as_array().into_iter().flatten() {
            if e["status"] == "cancelled" {
                continue;
            }
            let private = &e["extendedProperties"]["private"];
            if let (Some(id), Some(key)) = (e["id"].as_str(), private["managed_key"].as_str()) {
                found.push(Remote {
                    id: id.into(),
                    etag: e["etag"].as_str().unwrap_or_default().into(),
                    managed_key: key.into(),
                });
            }
        }
        match body["nextPageToken"].as_str() {
            Some(t) => page = Some(t.into()),
            None => return Ok(found),
        }
    }
}

fn when(timing: &EventTiming) -> (Value, Value) {
    let stamp = |t: &DateTime<Utc>| json!({ "dateTime": t.to_rfc3339(), "timeZone": "UTC" });
    match timing {
        EventTiming::AllDay(d) => (
            json!({ "date": d.to_string() }),
            json!({ "date": (*d + Duration::days(1)).to_string() }),
        ),
        EventTiming::Timed { start, end } => (stamp(start), stamp(end)),
    }
}

fn body(e: &DesiredEvent) -> Value {
    let (start, end) = when(&e.timing);
    let overrides: Vec<Value> = e
        .reminder_minutes
        .map(|m| json!({ "method": "popup", "minutes": m }))
        .into_iter()
        .collect();
    json!({
        "summary": e.summary,
        "description": e.description,
        "start": start,
        "end": end,
        "reminders": { "useDefault": false, "overrides": overrides },
        "extendedProperties": { "private": { "taskapp": "1", "managed_key": e.managed_key } },
    })
}

/// `(id, etag)` of the stored event.
fn stored(v: &Value) -> Result<(String, String)> {
    let text = |k: &str| v[k].as_str().map(str::to_owned);
    text("id")
        .zip(text("etag"))
        .ok_or_else(|| bad("the event response had no id or etag"))
}

pub(crate) async fn insert(
    g: &Google,
    token: &str,
    cal: &str,
    e: &DesiredEvent,
) -> Result<(String, String)> {
    let req = reqwest::Client::new()
        .post(url(g, &["calendars", cal, "events"])?)
        .bearer_auth(token)
        .json(&body(e));
    let (status, v) = send(req).await?;
    stored(&ok(status, v)?)
}

/// Overwrites the event. `None` means it is gone and the caller recreates it.
pub(crate) async fn update(
    g: &Google,
    token: &str,
    cal: &str,
    id: &str,
    etag: Option<&str>,
    e: &DesiredEvent,
) -> Result<Option<(String, String)>> {
    let put = |etag: Option<&str>| -> Result<RequestBuilder> {
        let mut req = reqwest::Client::new()
            .request(Method::PUT, url(g, &["calendars", cal, "events", id])?)
            .bearer_auth(token)
            .json(&body(e));
        if let Some(t) = etag {
            req = req.header("If-Match", t);
        }
        Ok(req)
    };
    let (mut status, mut v) = send(put(etag)?).await?;
    if status == StatusCode::PRECONDITION_FAILED {
        // Someone edited it after we listed it; the app wins, so write again without the precondition.
        (status, v) = send(put(None)?).await?;
    }
    if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
        return Ok(None);
    }
    stored(&ok(status, v)?).map(Some)
}

pub(crate) async fn delete(g: &Google, token: &str, cal: &str, id: &str) -> Result<()> {
    let req = reqwest::Client::new()
        .delete(url(g, &["calendars", cal, "events", id])?)
        .bearer_auth(token);
    let (status, v) = send(req).await?;
    if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
        return Ok(());
    }
    ok(status, v).map(|_| ())
}
