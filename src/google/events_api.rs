use anyhow::{Context, Result};
use reqwest::StatusCode;

use crate::google::types::{GoogleEvent, GoogleEventListResponse};

const MAX_ATTEMPTS: usize = 4;
const SEND_UPDATES_POLICY: &str = "all";

#[derive(Debug, Clone)]
pub struct GoogleApiError {
    pub status: StatusCode,
    pub message: String,
}

impl std::fmt::Display for GoogleApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.message)
    }
}

impl std::error::Error for GoogleApiError {}

impl GoogleApiError {
    pub fn is_sync_token_expired(&self) -> bool {
        self.status == StatusCode::GONE
    }

    pub fn is_rate_limited(&self) -> bool {
        self.status == StatusCode::TOO_MANY_REQUESTS
            || (self.status == StatusCode::FORBIDDEN
                && (self.message.contains("rateLimitExceeded")
                    || self.message.contains("quotaExceeded")
                    || self.message.contains("userRateLimitExceeded")))
    }

    pub fn is_precondition_failed(&self) -> bool {
        self.status == StatusCode::PRECONDITION_FAILED
    }

    pub fn is_not_found(&self) -> bool {
        self.status == StatusCode::NOT_FOUND
    }
}

pub async fn list_events(
    access_token: &str,
    google_calendar_id: &str,
    sync_token: Option<&str>,
    page_token: Option<&str>,
) -> Result<GoogleEventListResponse, GoogleApiError> {
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events",
        urlencode(google_calendar_id)
    );

    let mut query = vec![("maxResults", "2500"), ("showDeleted", "true")];
    if let Some(sync_token) = sync_token {
        query.push(("syncToken", sync_token));
    } else {
        query.push(("singleEvents", "false"));
    }
    if let Some(page_token) = page_token {
        query.push(("pageToken", page_token));
    }

    send_json_request(
        reqwest::Client::new()
            .get(url)
            .bearer_auth(access_token)
            .query(&query),
    )
    .await
}

pub async fn get_event(
    access_token: &str,
    google_calendar_id: &str,
    google_event_id: &str,
) -> Result<GoogleEvent, GoogleApiError> {
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
        urlencode(google_calendar_id),
        urlencode(google_event_id)
    );

    send_json_request(
        reqwest::Client::new()
            .get(url)
            .bearer_auth(access_token)
            .query(&[("showDeleted", "true")]),
    )
    .await
}

pub async fn insert_event(
    access_token: &str,
    google_calendar_id: &str,
    body: &serde_json::Value,
) -> Result<GoogleEvent, GoogleApiError> {
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events",
        urlencode(google_calendar_id)
    );

    send_json_request(
        reqwest::Client::new()
            .post(url)
            .bearer_auth(access_token)
            .query(&[("sendUpdates", SEND_UPDATES_POLICY)])
            .json(body),
    )
    .await
}

pub async fn patch_event(
    access_token: &str,
    google_calendar_id: &str,
    google_event_id: &str,
    body: &serde_json::Value,
    etag: Option<&str>,
) -> Result<GoogleEvent, GoogleApiError> {
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
        urlencode(google_calendar_id),
        urlencode(google_event_id)
    );
    let mut request = reqwest::Client::new()
        .patch(url)
        .bearer_auth(access_token)
        .query(&[("sendUpdates", SEND_UPDATES_POLICY)])
        .json(body);
    if let Some(etag) = etag {
        request = request.header("If-Match", etag);
    }
    send_json_request(request).await
}

pub async fn delete_event(
    access_token: &str,
    google_calendar_id: &str,
    google_event_id: &str,
    etag: Option<&str>,
) -> Result<(), GoogleApiError> {
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
        urlencode(google_calendar_id),
        urlencode(google_event_id)
    );
    let mut request = reqwest::Client::new()
        .delete(url)
        .bearer_auth(access_token)
        .query(&[("sendUpdates", SEND_UPDATES_POLICY)]);
    if let Some(etag) = etag {
        request = request.header("If-Match", etag);
    }
    send_empty_request(request).await
}

async fn send_json_request<T>(request: reqwest::RequestBuilder) -> Result<T, GoogleApiError>
where
    T: serde::de::DeserializeOwned,
{
    let mut attempt = 0usize;
    loop {
        let response = request
            .try_clone()
            .expect("google request must be clonable")
            .send()
            .await
            .map_err(|error| GoogleApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: error.to_string(),
            })?;
        let status = response.status();

        if status.is_success() {
            return response.json::<T>().await.map_err(|error| GoogleApiError {
                status,
                message: error.to_string(),
            });
        }

        let error = parse_google_error(status, response)
            .await
            .unwrap_or_else(|parse_error| GoogleApiError {
                status,
                message: parse_error.to_string(),
            });
        if error.is_rate_limited() && attempt + 1 < MAX_ATTEMPTS {
            attempt += 1;
            backoff(attempt).await;
            continue;
        }
        return Err(error);
    }
}

async fn send_empty_request(request: reqwest::RequestBuilder) -> Result<(), GoogleApiError> {
    let mut attempt = 0usize;
    loop {
        let response = request
            .try_clone()
            .expect("google request must be clonable")
            .send()
            .await
            .map_err(|error| GoogleApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: error.to_string(),
            })?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        let error = parse_google_error(status, response)
            .await
            .unwrap_or_else(|parse_error| GoogleApiError {
                status,
                message: parse_error.to_string(),
            });
        if error.is_rate_limited() && attempt + 1 < MAX_ATTEMPTS {
            attempt += 1;
            backoff(attempt).await;
            continue;
        }
        return Err(error);
    }
}

async fn parse_google_error(
    status: StatusCode,
    response: reqwest::Response,
) -> Result<GoogleApiError> {
    let payload = response
        .json::<serde_json::Value>()
        .await
        .context("google API error JSON parse failed")?;
    let message = payload["error"]["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| payload.to_string());
    let reasons = payload["error"]["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .filter_map(|error| error["reason"].as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let message = if reasons.is_empty() {
        message
    } else {
        format!("{} ({})", message, reasons.join(", "))
    };
    Ok(GoogleApiError { status, message })
}

async fn backoff(attempt: usize) {
    let millis = 250u64.saturating_mul(2u64.saturating_pow(attempt as u32));
    tokio::time::sleep(tokio::time::Duration::from_millis(millis)).await;
}

fn urlencode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' || c == '@'
            {
                vec![c]
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}
