use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::distributions::{Alphanumeric, DistString};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use urlencoding::decode;

pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
const REDIRECT_URI_ENCODED: &str = "http%3A%2F%2F127.0.0.1%3A8888%2Fcallback";
const CALLBACK_PORT: u16 = 8888;

const SCOPES: &str = concat!(
    "user-read-private%20",
    "user-modify-playback-state%20",
    "streaming%20",
    "user-read-playback-state%20",
    "playlist-read-private%20",
    "playlist-read-collaborative"
);

pub fn generate_state() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0xc0ffee);
    format!("audia{nanos:010}")
}

pub fn generate_code_verifier() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 64)
}

pub fn code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn auth_url(client_id: &str, state: &str, code_challenge: &str) -> String {
    format!(
        "https://accounts.spotify.com/authorize\
         ?client_id={client_id}\
         &response_type=code\
         &redirect_uri={REDIRECT_URI_ENCODED}\
         &scope={SCOPES}\
         &state={state}\
         &code_challenge_method=S256\
         &code_challenge={code_challenge}"
    )
}

/// Blocks the calling thread until Spotify redirects the user to 127.0.0.1:8888/callback.
/// Returns the authorization code on success.
pub fn wait_for_callback(expected_state: &str) -> Result<String, String> {
    wait_for_callback_timeout(expected_state, Duration::from_secs(180))
}

pub fn wait_for_callback_timeout(
    expected_state: &str,
    timeout: Duration,
) -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{CALLBACK_PORT}"))
        .map_err(|err| format!("Failed to bind 127.0.0.1:{CALLBACK_PORT}: {err}"))?;

    listener
        .set_nonblocking(true)
        .map_err(|err| format!("Failed to configure callback listener: {err}"))?;

    let deadline = Instant::now() + timeout;

    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(conn) => break conn,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(format!(
                        "Timed out waiting for OAuth callback on {} after {} seconds",
                        REDIRECT_URI,
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(format!("Failed to accept OAuth callback: {err}")),
        }
    };

    let mut request_line = String::new();
    BufReader::new(&stream)
        .read_line(&mut request_line)
        .map_err(|err| format!("Failed to read OAuth callback request: {err}"))?;

    // Parse: GET /callback?code=XXX&state=YYY HTTP/1.1
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let query = path.split('?').nth(1).unwrap_or("");

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;

    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix("code=") {
            code = Some(
                decode(v)
                    .map_err(|err| format!("Failed to decode OAuth code: {err}"))?
                    .into_owned(),
            );
        } else if let Some(v) = pair.strip_prefix("state=") {
            state = Some(
                decode(v)
                    .map_err(|err| format!("Failed to decode OAuth state: {err}"))?
                    .into_owned(),
            );
        } else if let Some(v) = pair.strip_prefix("error=") {
            error = Some(
                decode(v)
                    .map_err(|err| format!("Failed to decode OAuth error: {err}"))?
                    .into_owned(),
            );
        }
    }

    let body = if code.is_some() && error.is_none() {
        "<html><body style='font-family:sans-serif;text-align:center;padding:60px'>\
         <h2>&#10003; Audia: Login successful!</h2>\
         <p>You can close this tab and return to the app.</p></body></html>"
    } else {
        "<html><body style='font-family:sans-serif;text-align:center;padding:60px'>\
         <h2>&#10007; Audia: Login failed.</h2>\
         <p>Please return to the app and try again.</p></body></html>"
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).ok();
    drop(stream);

    if let Some(err) = error {
        return Err(format!("Spotify authorization denied: {err}"));
    }

    if state.as_deref() != Some(expected_state) {
        return Err("OAuth state mismatch guard triggered — aborting login".to_string());
    }

    code.ok_or_else(|| "No authorization code in Spotify callback".to_string())
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    #[serde(rename = "token_type")]
    pub _token_type: String,
}

pub async fn exchange_code(
    client_id: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse, String> {
    let http = Client::new();

    let response = http
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|err| format!("Token exchange request failed: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed ({status}): {body}"));
    }

    response
        .json::<TokenResponse>()
        .await
        .map_err(|err| format!("Invalid token exchange response: {err}"))
}

pub async fn refresh_access_token(
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse, String> {
    let http = Client::new();

    let response = http
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|err| format!("Token refresh request failed: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token refresh failed ({status}): {body}"));
    }

    response
        .json::<TokenResponse>()
        .await
        .map_err(|err| format!("Invalid token refresh response: {err}"))
}
