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

include!("auth/types.rs");
include!("auth/client.rs");
include!("auth/oauth.rs");
include!("auth/storage.rs");

#[cfg(test)]
include!("auth/tests.rs");
