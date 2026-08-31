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
