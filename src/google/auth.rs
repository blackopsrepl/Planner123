use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KEYRING_SERVICE: &str = "solverforge-calendar";
const KEYRING_CLIENT_ID_KEY: &str = "google_client_id";
const KEYRING_CLIENT_SECRET_KEY: &str = "google_client_secret";
const KEYRING_REFRESH_TOKEN_KEY: &str = "google_refresh_token";
pub const GOOGLE_CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoogleAuthState {
    Disconnected,
    Connected,
    NeedsReauth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAuthStatus {
    pub state: GoogleAuthState,
    pub has_client_id: bool,
    pub has_client_secret: bool,
    pub has_refresh_token: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleSavedCredentials {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/* Opaque client handle — used by the sync module to make API calls. */
#[derive(Debug, Clone)]
pub struct GoogleClient {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub refresh_token: String,
}

impl GoogleClient {
    pub fn from_keyring() -> Option<Self> {
        let credentials = load_saved_credentials()?;
        let refresh_token = read_keyring(KEYRING_REFRESH_TOKEN_KEY)?;
        Some(Self {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            refresh_token,
        })
    }

    pub fn is_configured() -> bool {
        read_keyring(KEYRING_REFRESH_TOKEN_KEY).is_some()
    }

    pub fn saved_credentials() -> Option<GoogleSavedCredentials> {
        load_saved_credentials()
    }

    pub fn save_credentials(client_id: &str, client_secret: Option<&str>) -> Result<()> {
        write_keyring(KEYRING_CLIENT_ID_KEY, client_id)?;
        if let Some(client_secret) = client_secret.filter(|value| !value.trim().is_empty()) {
            write_keyring(KEYRING_CLIENT_SECRET_KEY, client_secret)?;
        } else {
            let _ = delete_keyring(KEYRING_CLIENT_SECRET_KEY);
        }
        Ok(())
    }

    pub fn save_refresh_token(refresh_token: &str) -> Result<()> {
        write_keyring(KEYRING_REFRESH_TOKEN_KEY, refresh_token)
    }

    pub fn clear_refresh_token() -> Result<()> {
        let _ = delete_keyring(KEYRING_REFRESH_TOKEN_KEY);
        Ok(())
    }

    pub fn logout() -> Result<GoogleAuthStatus> {
        Self::clear_refresh_token()?;
        Ok(auth_status())
    }

    pub async fn refresh_access_token(&self) -> Result<String> {
        let mut params = vec![
            ("client_id", self.client_id.as_str()),
            ("refresh_token", self.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];
        if let Some(client_secret) = self
            .client_secret
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            params.push(("client_secret", client_secret));
        }

        let response = reqwest::Client::new()
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .context("token refresh request failed")?;
        let status = response.status();
        let response = response
            .json::<serde_json::Value>()
            .await
            .context("token refresh JSON parse failed")?;

        if let Some(access_token) = response["access_token"].as_str() {
            return Ok(access_token.to_string());
        }

        if response["error"].as_str() == Some("invalid_grant") {
            let _ = Self::clear_refresh_token();
            bail!("google authentication expired; reconnect required");
        }

        bail!(
            "token refresh failed with {}: {}",
            status,
            response["error_description"]
                .as_str()
                .or(response["error"].as_str())
                .unwrap_or("unknown error")
        )
    }
}

pub fn auth_status() -> GoogleAuthStatus {
    let has_client_id = read_keyring(KEYRING_CLIENT_ID_KEY).is_some();
    let has_client_secret = read_keyring(KEYRING_CLIENT_SECRET_KEY).is_some();
    let has_refresh_token = read_keyring(KEYRING_REFRESH_TOKEN_KEY).is_some();
    let state = if has_refresh_token && has_client_id {
        GoogleAuthState::Connected
    } else if has_client_id {
        GoogleAuthState::NeedsReauth
    } else {
        GoogleAuthState::Disconnected
    };

    GoogleAuthStatus {
        state,
        has_client_id,
        has_client_secret,
        has_refresh_token,
    }
}

pub async fn authorize_and_persist(
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<GoogleClient> {
    GoogleClient::save_credentials(client_id, client_secret)?;
    let refresh_token = run_oauth_flow(client_id, client_secret).await?;
    GoogleClient::save_refresh_token(&refresh_token)?;
    Ok(GoogleClient {
        client_id: client_id.to_string(),
        client_secret: client_secret.map(str::to_string),
        refresh_token,
    })
}

pub async fn run_oauth_flow(client_id: &str, client_secret: Option<&str>) -> Result<String> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .context("cannot bind OAuth callback server on a loopback port")?;
    let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let state = random_token();
    let code_verifier = random_token();
    let auth_url = build_authorization_url(&OAuthRequest {
        client_id,
        redirect_uri: &redirect_uri,
        state: &state,
        scope: GOOGLE_CALENDAR_SCOPE,
        code_challenge: &pkce_challenge(&code_verifier),
    });

    open::that(&auth_url).context("cannot open the system browser for Google authorization")?;

    let callback = tokio::task::spawn_blocking(move || -> Result<CallbackParams> {
        let (mut stream, _) = listener.accept().context("OAuth callback not received")?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let callback = extract_callback_params(&request_line)?;

        let response = if callback.error.is_some() {
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n\
             <html><body style='background:#0B0C16;color:#ff7a90;font-family:monospace'>\
             <h2>SolverForge Calendar</h2><p>Authorization failed. Return to the app and try again.</p>\
             </body></html>"
        } else {
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
             <html><body style='background:#0B0C16;color:#82FB9C;font-family:monospace'>\
             <h2>SolverForge Calendar</h2><p>Authorization complete. You can close this tab.</p>\
             </body></html>"
        };
        let _ = stream.write_all(response.as_bytes());
        Ok(callback)
    })
    .await
    .context("OAuth callback worker failed")??;

    validate_callback_state(&state, callback.state.as_deref())?;
    if let Some(error) = callback.error {
        bail!("google authorization failed: {}", error);
    }

    let code = callback
        .code
        .ok_or_else(|| anyhow!("no authorization code in OAuth callback"))?;
    let token_response = exchange_code_for_token(
        client_id,
        client_secret,
        &code,
        &redirect_uri,
        &code_verifier,
    )
    .await?;
    let refresh_token = token_response["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow!("no refresh_token in token response"))?;
    Ok(refresh_token.to_string())
}

#[derive(Debug)]
struct OAuthRequest<'a> {
    client_id: &'a str,
    redirect_uri: &'a str,
    state: &'a str,
    scope: &'a str,
    code_challenge: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn build_authorization_url(request: &OAuthRequest<'_>) -> String {
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?client_id={}\
         &redirect_uri={}\
         &response_type=code\
         &scope={}\
         &access_type=offline\
         &prompt=consent\
         &state={}\
         &code_challenge={}\
         &code_challenge_method=S256",
        urlencode(request.client_id),
        urlencode(request.redirect_uri),
        urlencode(request.scope),
        urlencode(request.state),
        urlencode(request.code_challenge),
    )
}

fn extract_callback_params(request_line: &str) -> Result<CallbackParams> {
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("invalid OAuth callback request line"))?;
    let query = path.split('?').nth(1).unwrap_or_default();

    let mut callback = CallbackParams {
        code: None,
        state: None,
        error: None,
    };

    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        match key {
            "code" => callback.code = Some(urldecode(value)),
            "state" => callback.state = Some(urldecode(value)),
            "error" => callback.error = Some(urldecode(value)),
            _ => {}
        }
    }

    Ok(callback)
}

fn validate_callback_state(expected: &str, actual: Option<&str>) -> Result<()> {
    match actual {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => bail!("google authorization callback state mismatch"),
        None => bail!("missing state in google authorization callback"),
    }
}

async fn exchange_code_for_token(
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<serde_json::Value> {
    let mut params = vec![
        ("code", code),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];
    if let Some(client_secret) = client_secret.filter(|value| !value.trim().is_empty()) {
        params.push(("client_secret", client_secret));
    }

    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .context("token exchange HTTP request failed")?;
    let status = response.status();
    let response = response
        .json::<serde_json::Value>()
        .await
        .context("token exchange JSON parse failed")?;
    if !status.is_success() {
        bail!(
            "token exchange failed with {}: {}",
            status,
            response["error_description"]
                .as_str()
                .or(response["error"].as_str())
                .unwrap_or("unknown error")
        );
    }
    Ok(response)
}

fn pkce_challenge(code_verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()))
}

fn random_token() -> String {
    let first = uuid::Uuid::new_v4().simple().to_string();
    let second = uuid::Uuid::new_v4().simple().to_string();
    format!("{}{}", first, second)
}

fn load_saved_credentials() -> Option<GoogleSavedCredentials> {
    let client_id = read_keyring(KEYRING_CLIENT_ID_KEY)?;
    let client_secret = read_keyring(KEYRING_CLIENT_SECRET_KEY);
    Some(GoogleSavedCredentials {
        client_id,
        client_secret,
    })
}

fn read_keyring(key: &str) -> Option<String> {
    Entry::new(&keyring_service(), key)
        .ok()?
        .get_password()
        .ok()
}

fn write_keyring(key: &str, value: &str) -> Result<()> {
    Entry::new(&keyring_service(), key)
        .context("keyring entry creation failed")?
        .set_password(value)
        .context("keyring write failed")
}

fn delete_keyring(key: &str) -> Result<()> {
    if let Ok(entry) = Entry::new(&keyring_service(), key) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

fn keyring_service() -> String {
    std::env::var("SOLVERFORGE_CALENDAR_TEST_KEYRING_SERVICE")
        .unwrap_or_else(|_| KEYRING_SERVICE.to_string())
}

fn urlencode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                vec![c]
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}

fn urldecode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        build_authorization_url, extract_callback_params, pkce_challenge, validate_callback_state,
        OAuthRequest,
    };

    #[test]
    fn authorization_url_includes_pkce_state_and_loopback_redirect() {
        let url = build_authorization_url(&OAuthRequest {
            client_id: "client-id",
            redirect_uri: "http://127.0.0.1:8123",
            state: "state-123",
            scope: "scope-a scope-b",
            code_challenge: "challenge-123",
        });

        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A8123"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("code_challenge=challenge-123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
    }

    #[test]
    fn callback_parser_extracts_code_state_and_error() {
        let callback =
            extract_callback_params("GET /?code=abc123&state=expected&error=denied HTTP/1.1")
                .unwrap();

        assert_eq!(callback.code.as_deref(), Some("abc123"));
        assert_eq!(callback.state.as_deref(), Some("expected"));
        assert_eq!(callback.error.as_deref(), Some("denied"));
    }

    #[test]
    fn callback_state_validation_rejects_mismatches() {
        validate_callback_state("expected", Some("expected")).unwrap();
        let err = validate_callback_state("expected", Some("wrong")).unwrap_err();
        assert!(err.to_string().contains("state mismatch"));
    }

    #[test]
    fn pkce_challenge_is_url_safe() {
        let challenge = pkce_challenge("0123456789abcdef0123456789abcdef0123456789");
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
    }
}
