use chrono::{Duration, Utc};
use serde_json::{Value, json};
use taskapp_core::{
    ChangeSource, Db, Due, IssuePatch, NewIssue, SyncState, SyncStatus, blocks, issues, settings,
    statuses,
};
use taskapp_gcal::{
    auth::connect_with,
    client::{DISCONNECTED, Google, list_calendars_with},
    reconcile::{ReconcileReport, reconcile, reconcile_with},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, header, method, path},
};

const REFRESH: &str = "refresh-secret-123";
const EVENTS: &str = "/calendar/v3/calendars/cal1/events";

fn google(s: &MockServer) -> Google {
    Google {
        api: format!("{}/calendar/v3", s.uri()),
        auth_url: format!("{}/auth", s.uri()),
        token_url: format!("{}/token", s.uri()),
        client_id: "client".into(),
        client_secret: "secret".into(),
    }
}

async fn server() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at"})))
        .mount(&s)
        .await;
    s
}

async fn db() -> Db {
    let db = Db::open_in_memory().await.expect("db");
    settings::set(&db, "gcal_enabled", &true)
        .await
        .expect("enabled");
    settings::set(&db, "gcal_calendar_id", &"cal1")
        .await
        .expect("calendar");
    db
}

/// One open issue due at a timed instant, so it maps to exactly one event, `issue:<id>`.
async fn timed_issue(db: &Db) -> i64 {
    let due = Due::DateTime {
        utc: Utc::now() + Duration::days(3),
        tz: "UTC".into(),
    };
    let new = NewIssue {
        title: "Pay rent".into(),
        due: Some(due),
        ..NewIssue::default()
    };
    issues::create(db, new, ChangeSource::Ui)
        .await
        .expect("issue")
        .id
        .0
}

fn listing(items: &[(&str, &str, &str)]) -> ResponseTemplate {
    let items: Vec<Value> = items
        .iter()
        .map(|(id, etag, key)| {
            json!({"id": id, "etag": etag, "status": "confirmed",
                   "extendedProperties": {"private": {"taskapp": "1", "managed_key": key}}})
        })
        .collect();
    ResponseTemplate::new(200).set_body_json(json!({ "items": items }))
}

async fn mount_list(s: &MockServer, items: &[(&str, &str, &str)]) {
    Mock::given(method("GET"))
        .and(path(EVENTS))
        .respond_with(listing(items))
        .mount(s)
        .await;
}

fn stored(id: &str, etag: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({"id": id, "etag": etag}))
}

/// Creates the issue's event on a first server and returns its key.
async fn synced(db: &Db) -> String {
    let key = format!("issue:{}", timed_issue(db).await);
    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("g1", "e1"))
        .expect(1)
        .mount(&s)
        .await;
    reconcile_with(db, &google(&s), REFRESH)
        .await
        .expect("first sync");
    key
}

async fn row_ids(db: &Db) -> Vec<String> {
    sqlx::query_scalar("SELECT google_event_id FROM gcal_events ORDER BY 1")
        .fetch_all(&db.pool)
        .await
        .expect("rows")
}

#[tokio::test]
async fn reconcile_creates_from_scratch() {
    let db = db().await;
    let id = timed_issue(&db).await;
    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .and(header("authorization", "Bearer at"))
        .and(body_string_contains(format!(
            "\"managed_key\":\"issue:{id}\""
        )))
        .and(body_string_contains("\"taskapp\":\"1\""))
        .and(body_string_contains("\"minutes\":0"))
        .respond_with(stored("g1", "e1"))
        .expect(1)
        .mount(&s)
        .await;
    let report = reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    assert_eq!(
        report,
        ReconcileReport {
            created: 1,
            updated: 0,
            deleted: 0
        }
    );
    assert_eq!(row_ids(&db).await, ["g1"]);
    let status: SyncStatus = settings::get(&db, "sync_status")
        .await
        .expect("get")
        .expect("set");
    assert!(matches!(status.gcal, SyncState::Ok { .. }));
}

#[tokio::test]
async fn refresh_token_is_never_stored_or_sent_to_the_calendar() {
    let db = db().await;
    synced(&db).await;
    let dump: String = sqlx::query_scalar(
        "SELECT (SELECT group_concat(value) FROM settings) || (SELECT group_concat(managed_key || etag || last_synced_hash) FROM gcal_events)
         || (SELECT coalesce(group_concat(new_value), '') FROM changes)",
    )
    .fetch_one(&db.pool)
    .await
    .expect("dump");
    assert!(!dump.contains(REFRESH));

    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("g2", "e"))
        .mount(&s)
        .await;
    reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    for r in s.received_requests().await.expect("recorded") {
        let on_api = r.url.path() != "/token";
        let seen = format!("{:?}{}", r.headers, String::from_utf8_lossy(&r.body));
        assert!(
            !(on_api && seen.contains(REFRESH)),
            "refresh token reached {}",
            r.url
        );
    }
}

#[tokio::test]
async fn reconcile_recreates_deleted_event() {
    let db = db().await;
    synced(&db).await;
    // The event was deleted in Google: the listing no longer has it.
    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("g2", "e2"))
        .expect(1)
        .mount(&s)
        .await;
    let report = reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    assert_eq!(report.created, 1);
    assert_eq!(row_ids(&db).await, ["g2"]);
}

#[tokio::test]
async fn update_of_event_deleted_meanwhile_recreates_it() {
    let db = db().await;
    let key = synced(&db).await;
    let s = server().await;
    mount_list(&s, &[("g1", "edited", &key)]).await;
    Mock::given(method("PUT"))
        .and(path(format!("{EVENTS}/g1")))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("g3", "e3"))
        .expect(1)
        .mount(&s)
        .await;
    reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    assert_eq!(row_ids(&db).await, ["g3"]);
}

#[tokio::test]
async fn reconcile_overwrites_manual_edit() {
    let db = db().await;
    let key = synced(&db).await;

    let untouched = server().await;
    mount_list(&untouched, &[("g1", "e1", &key)]).await;
    let report = reconcile_with(&db, &google(&untouched), REFRESH)
        .await
        .expect("no-op");
    assert_eq!(report, ReconcileReport::default());

    // Someone edited it in Google: the etag moved on.
    let s = server().await;
    mount_list(&s, &[("g1", "e2", &key)]).await;
    Mock::given(method("PUT"))
        .and(path(format!("{EVENTS}/g1")))
        .and(header("if-match", "e2"))
        .respond_with(stored("g1", "e3"))
        .expect(1)
        .mount(&s)
        .await;
    let report = reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("overwrite");
    assert_eq!(report.updated, 1);
}

#[tokio::test]
async fn etag_mismatch_is_overwritten_not_failed() {
    let db = db().await;
    let key = synced(&db).await;
    let s = server().await;
    mount_list(&s, &[("g1", "e2", &key)]).await;
    Mock::given(method("PUT"))
        .and(path(format!("{EVENTS}/g1")))
        .and(header("if-match", "e2"))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("PUT"))
        .and(path(format!("{EVENTS}/g1")))
        .respond_with(stored("g1", "e4"))
        .expect(1)
        .mount(&s)
        .await;
    let report = reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("overwrite");
    assert_eq!(report.updated, 1);
}

#[tokio::test]
async fn completed_issue_event_deleted() {
    let db = db().await;
    let key = synced(&db).await;
    let done = statuses::list(&db)
        .await
        .expect("statuses")
        .into_iter()
        .find(|s| s.name == "Done")
        .expect("Done");
    let patch = IssuePatch {
        status_id: Some(done.id),
        ..IssuePatch::default()
    };
    let id = key.trim_start_matches("issue:").parse().expect("id");
    issues::update(&db, taskapp_core::IssueId(id), patch, ChangeSource::Ui)
        .await
        .expect("complete");

    let s = server().await;
    mount_list(&s, &[("g1", "e1", &key)]).await;
    Mock::given(method("DELETE"))
        .and(path(format!("{EVENTS}/g1")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&s)
        .await;
    let report = reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    assert_eq!(report.deleted, 1);
    assert_eq!(row_ids(&db).await, Vec::<String>::new());
}

#[tokio::test]
async fn revoked_token_sets_error_state() {
    let db = db().await;
    timed_issue(&db).await;
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_grant"})))
        .mount(&s)
        .await;
    let err = reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect_err("fails");
    assert!(err.to_string().contains(DISCONNECTED));
    let status: SyncStatus = settings::get(&db, "sync_status")
        .await
        .expect("get")
        .expect("set");
    match status.gcal {
        SyncState::Error { message, .. } => assert_eq!(message, DISCONNECTED),
        other => panic!("expected the error state, got {other:?}"),
    }
    assert!(!err.to_string().contains(REFRESH));
}

#[tokio::test]
async fn disabled_setting_is_noop() {
    let db = db().await;
    settings::set(&db, "gcal_enabled", &false)
        .await
        .expect("off");
    timed_issue(&db).await;
    assert_eq!(
        reconcile(&db).await.expect("noop"),
        ReconcileReport::default()
    );
    assert_eq!(row_ids(&db).await, Vec::<String>::new());
}

#[tokio::test]
async fn other_calendars_are_never_touched() {
    let db = db().await;
    sqlx::query(
        "INSERT INTO gcal_events (google_event_id, calendar_id, managed_key, etag, last_synced_hash) VALUES ('old', 'other', 'issue:999', 'e', 'h')",
    )
    .execute(&db.pool)
    .await
    .expect("seed");
    timed_issue(&db).await;
    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("g1", "e1"))
        .mount(&s)
        .await;
    reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    assert_eq!(row_ids(&db).await, ["g1", "old"]);
    for r in s.received_requests().await.expect("recorded") {
        assert!(
            !r.url.path().contains("other") && !r.url.path().contains("old"),
            "touched {}",
            r.url
        );
    }
}

#[tokio::test]
async fn connect_runs_pkce_and_returns_the_refresh_token() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=the-code"))
        .and(body_string_contains("code_verifier="))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token": "at", "refresh_token": "rt"})),
        )
        .expect(1)
        .mount(&s)
        .await;
    let token = connect_with(&google(&s), |url| visit(url, "the-code", None))
        .await
        .expect("connect");
    assert_eq!(token, "rt");
    let consent = s.received_requests().await.expect("recorded");
    assert_eq!(consent.len(), 1);
}

#[tokio::test]
async fn connect_rejects_a_forged_state() {
    let s = MockServer::start().await;
    let err = connect_with(&google(&s), |url| visit(url, "c", Some("forged")))
        .await
        .expect_err("fails");
    assert!(err.to_string().contains("state mismatch"));
}

/// Plays the browser: reads the consent URL and follows the redirect back to the loopback listener.
fn visit(consent: &str, code: &str, state: Option<&str>) {
    let url = reqwest::Url::parse(consent).expect("consent url");
    let q = |k: &str| {
        url.query_pairs()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.into_owned())
            .expect("param")
    };
    assert!(
        q("scope").contains("calendar.events")
            && q("scope").contains("calendar.calendarlist.readonly")
    );
    assert_eq!(q("code_challenge_method"), "S256");
    let back = format!(
        "{}/?code={code}&state={}",
        q("redirect_uri"),
        state.map_or_else(|| q("state"), str::to_owned)
    );
    tokio::spawn(async move { reqwest::get(back).await.expect("redirect") });
}

#[tokio::test]
async fn lists_writable_calendars() {
    let s = server().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"items": [{"id": "cal1", "summary": "Tasks"}]})),
        )
        .mount(&s)
        .await;
    let got = list_calendars_with(&google(&s), REFRESH)
        .await
        .expect("list");
    assert_eq!(got, [("cal1".to_string(), "Tasks".to_string())]);
}

#[tokio::test]
async fn work_block_becomes_an_event() {
    let db = db().await;
    let id = timed_issue(&db).await;
    let start = Utc::now() + Duration::days(1);
    let block = blocks::create_block(
        &db,
        taskapp_core::IssueId(id),
        start,
        start + Duration::hours(1),
    )
    .await
    .expect("block");
    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .and(body_string_contains(format!(
            "\"managed_key\":\"block:{}\"",
            block.id.0
        )))
        .respond_with(stored("gb", "eb"))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("g1", "e1"))
        .mount(&s)
        .await;
    reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
    assert!(row_ids(&db).await.contains(&"gb".to_owned()));
}

#[tokio::test]
async fn digest_alert_uses_the_timezone_setting() {
    let db = db().await;
    settings::set(&db, "timezone", &"America/New_York")
        .await
        .expect("tz");
    let new = NewIssue {
        title: "Pay rent".into(),
        due: Some(Due::Date(
            chrono::NaiveDate::from_ymd_opt(2099, 1, 15).expect("date"),
        )),
        ..NewIssue::default()
    };
    issues::create(&db, new, ChangeSource::Ui)
        .await
        .expect("issue");
    let s = server().await;
    mount_list(&s, &[]).await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .and(body_string_contains("2099-01-15T13:00:00"))
        .respond_with(stored("ga", "ea"))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path(EVENTS))
        .respond_with(stored("gd", "ed"))
        .mount(&s)
        .await;
    reconcile_with(&db, &google(&s), REFRESH)
        .await
        .expect("reconcile");
}

#[tokio::test]
async fn token_exchange_sends_the_pkce_verifier_for_the_challenge() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token": "at", "refresh_token": "rt"})),
        )
        .mount(&s)
        .await;
    let challenge = std::sync::Mutex::new(String::new());
    connect_with(&google(&s), |url| {
        let u = reqwest::Url::parse(url).expect("url");
        let c = u
            .query_pairs()
            .find(|(k, _)| k == "code_challenge")
            .expect("challenge");
        *challenge.lock().expect("lock") = c.1.into_owned();
        visit(url, "c", None);
    })
    .await
    .expect("connect");
    let req = &s.received_requests().await.expect("recorded")[0];
    let body = String::from_utf8_lossy(&req.body).into_owned();
    let verifier = body
        .split('&')
        .find_map(|p| p.strip_prefix("code_verifier="))
        .expect("verifier");
    let expected = oauth2::PkceCodeChallenge::from_code_verifier_sha256(
        &oauth2::PkceCodeVerifier::new(verifier.to_owned()),
    );
    assert_eq!(expected.as_str(), *challenge.lock().expect("lock"));
}
