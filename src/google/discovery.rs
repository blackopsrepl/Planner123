use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::google::auth::GoogleClient;

const DEFAULT_CALENDAR_COLOR: &str = "#82FB9C";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredGoogleCalendar {
    pub google_id: String,
    pub name: String,
    pub color: String,
    pub primary: bool,
    pub access_role: Option<String>,
    pub writable: bool,
}

pub async fn discover_calendars(client: &GoogleClient) -> Result<Vec<DiscoveredGoogleCalendar>> {
    let access_token = client.refresh_access_token().await?;
    fetch_calendar_list(&access_token).await
}

async fn fetch_calendar_list(access_token: &str) -> Result<Vec<DiscoveredGoogleCalendar>> {
    let http = reqwest::Client::new();
    let url = "https://www.googleapis.com/calendar/v3/users/me/calendarList";

    let mut calendars = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut req = http
            .get(url)
            .bearer_auth(access_token)
            .query(&[("maxResults", "250")]);

        if let Some(page_token) = page_token.as_deref() {
            req = req.query(&[("pageToken", page_token)]);
        }

        let response = req
            .send()
            .await
            .context("Google Calendar calendar-list fetch failed")?;
        let status = response.status();
        let response = response
            .json::<serde_json::Value>()
            .await
            .context("Google Calendar calendar-list JSON parse failed")?;

        let page = parse_calendar_list_response(status, response)
            .context("Google Calendar calendar-list response rejected")?;

        calendars.extend(page.calendars);

        if let Some(next_page_token) = page.next_page_token {
            page_token = Some(next_page_token);
        } else {
            break;
        }
    }

    calendars.sort_by(|left, right| {
        right
            .primary
            .cmp(&left.primary)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.google_id.cmp(&right.google_id))
    });
    Ok(calendars)
}

#[derive(Debug)]
struct CalendarListPage {
    calendars: Vec<DiscoveredGoogleCalendar>,
    next_page_token: Option<String>,
}

fn parse_calendar_list_response(
    status: reqwest::StatusCode,
    response: serde_json::Value,
) -> Result<CalendarListPage> {
    if !status.is_success() {
        return Err(anyhow!(
            "Google Calendar calendar-list request returned {}: {}",
            status,
            response
        ));
    }

    Ok(CalendarListPage {
        calendars: parse_calendar_list_page(&response),
        next_page_token: response["nextPageToken"]
            .as_str()
            .map(|token| token.to_string()),
    })
}

fn parse_calendar_list_page(response: &serde_json::Value) -> Vec<DiscoveredGoogleCalendar> {
    response["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_calendar_item)
        .collect()
}

fn parse_calendar_item(item: &serde_json::Value) -> Option<DiscoveredGoogleCalendar> {
    if item["deleted"].as_bool().unwrap_or(false) {
        return None;
    }

    let google_id = item["id"].as_str()?.trim();
    if google_id.is_empty() {
        return None;
    }

    let name = item["summary"]
        .as_str()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .unwrap_or(google_id);
    let color = item["backgroundColor"]
        .as_str()
        .map(str::trim)
        .filter(|color| !color.is_empty())
        .unwrap_or(DEFAULT_CALENDAR_COLOR);

    Some(DiscoveredGoogleCalendar {
        google_id: google_id.to_string(),
        name: name.to_string(),
        color: color.to_string(),
        primary: item["primary"].as_bool().unwrap_or(false),
        access_role: item["accessRole"].as_str().map(str::to_string),
        writable: matches!(item["accessRole"].as_str(), Some("owner") | Some("writer")),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_calendar_list_page, parse_calendar_list_response, DiscoveredGoogleCalendar};

    #[test]
    fn parse_calendar_list_page_skips_deleted_rows_and_applies_defaults() {
        let parsed = parse_calendar_list_page(&serde_json::json!({
            "items": [
                {
                    "id": "primary@example.com",
                    "summary": "Primary",
                    "backgroundColor": "#112233",
                    "primary": true,
                    "accessRole": "writer"
                },
                {
                    "id": "fallback@example.com",
                    "summary": "   ",
                    "deleted": false
                },
                {
                    "id": "deleted@example.com",
                    "summary": "Deleted",
                    "deleted": true
                }
            ]
        }));

        assert_eq!(
            parsed,
            vec![
                DiscoveredGoogleCalendar {
                    google_id: "primary@example.com".to_string(),
                    name: "Primary".to_string(),
                    color: "#112233".to_string(),
                    primary: true,
                    access_role: Some("writer".to_string()),
                    writable: true,
                },
                DiscoveredGoogleCalendar {
                    google_id: "fallback@example.com".to_string(),
                    name: "fallback@example.com".to_string(),
                    color: "#82FB9C".to_string(),
                    primary: false,
                    access_role: None,
                    writable: false,
                },
            ]
        );
    }

    #[test]
    fn parse_calendar_list_response_rejects_google_error_payloads() {
        let err = parse_calendar_list_response(
            reqwest::StatusCode::UNAUTHORIZED,
            serde_json::json!({
                "error": {
                    "code": 401,
                    "message": "Request had invalid authentication credentials."
                }
            }),
        )
        .unwrap_err();

        assert!(err.to_string().contains("401 Unauthorized"));
    }
}
