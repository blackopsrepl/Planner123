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
