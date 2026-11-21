use std::time::Duration;

use oauth2::{CsrfToken, PkceCodeChallenge};
use reqwest::Url;
use taskapp_core::{Db, Error, Result, SyncState};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::client::{Google, flag, set_state};

const SCOPES: &str = "https://www.googleapis.com/auth/calendar.events \
                      https://www.googleapis.com/auth/calendar.calendarlist.readonly";
const KEYCHAIN_SERVICE: &str = "taskapp.gcal";
const WAIT_FOR_BROWSER: Duration = Duration::from_secs(300);

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(KEYCHAIN_SERVICE, "refresh_token")
        .map_err(|e| Error::Invalid(e.to_string()))
}

/// # Errors
///
/// Fails when no token is stored or the Keychain refuses.
pub(crate) fn load_token() -> Result<String> {
    entry()?
        .get_password()
        .map_err(|_| Error::Invalid(crate::client::DISCONNECTED.into()))
}

/// Browser OAuth with PKCE on a loopback redirect.
///
/// # Errors
///
/// Fails when the user cancels, the exchange fails, or the token cannot be stored.
pub async fn connect(db: &Db) -> Result<()> {
    let g = Google::production()?;
    let token = connect_with(&g, |url| {
        // The URL is Google's and was built by us, so it is safe to hand to the system opener.
        let _ = std::process::Command::new("open").arg(url).spawn();
    })
    .await;
    let token = flag(db, token).await?;
    entry()?
        .set_password(&token)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    set_state(
        db,
        SyncState::Ok {
            at: chrono::Utc::now(),
        },
    )
    .await
}

/// Runs the flow and returns the refresh token. `open` shows the consent URL to the user.
///
/// # Errors
///
/// As `connect`, except the token is not stored.
pub async fn connect_with(g: &Google, open: impl FnOnce(&str)) -> Result<String> {
    let bad = |m: &str| Error::Invalid(format!("Google sign-in failed: {m}"));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let state = CsrfToken::new_random();

    let mut consent = Url::parse(&g.auth_url).map_err(|e| bad(&e.to_string()))?;
    consent
        .query_pairs_mut()
        .append_pair("client_id", &g.client_id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", challenge.as_str())
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "offline")
        // Without this Google omits the refresh token on a repeat sign-in.
        .append_pair("prompt", "consent")
        .append_pair("state", state.secret());
    open(consent.as_str());

    let code = tokio::time::timeout(WAIT_FOR_BROWSER, await_code(&listener, state.secret()))
        .await
        .map_err(|_| bad("timed out waiting for the browser"))??;

    let resp = reqwest::Client::new()
        .post(&g.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("code_verifier", verifier.secret()),
            ("redirect_uri", &redirect),
            ("client_id", &g.client_id),
            ("client_secret", &g.client_secret),
        ])
        .send()
        .await
        .map_err(|e| bad(&e.without_url().to_string()))?;
    if !resp.status().is_success() {
        return Err(bad(&format!("token exchange returned {}", resp.status())));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| bad(&e.to_string()))?;
    body["refresh_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| bad("Google sent no refresh token"))
}

/// Serves the one redirect request and returns its `code`, after checking `state`.
async fn await_code(listener: &TcpListener, state: &str) -> Result<String> {
    let (mut sock, _) = listener.accept().await?;
    let mut buf = vec![0; 4096];
    let n = sock.read(&mut buf).await?;
    let head = String::from_utf8_lossy(&buf[..n]);
    let path = head.split_whitespace().nth(1).unwrap_or("/");
    let query = Url::parse(&format!("http://localhost{path}"))
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let param = |k: &str| {
        query
            .query_pairs()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.into_owned())
    };
    let reply = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n\
                 You can close this tab and return to the app.";
    sock.write_all(reply.as_bytes()).await?;
    if param("state").as_deref() != Some(state) {
        return Err(Error::Invalid(
            "Google sign-in failed: state mismatch".into(),
        ));
    }
    param("code").ok_or_else(|| Error::Invalid("Google sign-in failed: no code returned".into()))
}
