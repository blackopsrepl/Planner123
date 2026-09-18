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
             <h2>Planner123</h2><p>Authorization failed. Return to the app and try again.</p>\
             </body></html>"
        } else {
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
             <html><body style='background:#0B0C16;color:#82FB9C;font-family:monospace'>\
             <h2>Planner123</h2><p>Authorization complete. You can close this tab.</p>\
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
