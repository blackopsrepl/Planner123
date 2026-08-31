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
