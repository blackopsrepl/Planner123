use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::Event;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEventListResponse {
    #[serde(default)]
    pub items: Vec<GoogleEvent>,
    #[serde(rename = "nextSyncToken")]
    pub next_sync_token: Option<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEvent {
    pub id: Option<String>,
    pub etag: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<GoogleEventTime>,
    pub end: Option<GoogleEventTime>,
    pub recurrence: Option<Vec<String>>,
    pub updated: Option<String>,
    #[serde(rename = "recurringEventId")]
    pub recurring_event_id: Option<String>,
    #[serde(rename = "originalStartTime")]
    pub original_start_time: Option<GoogleEventTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

pub fn google_event_to_local(calendar_id: &str, event: &GoogleEvent) -> Result<Event> {
    let google_id = event
        .id
        .as_deref()
        .ok_or_else(|| anyhow!("google event has no id"))?
        .to_string();
    let timezone = event
        .start
        .as_ref()
        .and_then(|start| start.time_zone.clone())
        .or_else(|| event.end.as_ref().and_then(|end| end.time_zone.clone()))
        .unwrap_or_else(|| "UTC".to_string());
    let (start_at, all_day) = map_google_time(event.start.as_ref(), &timezone, true)?;
    let (end_at, _) = map_google_time(event.end.as_ref(), &timezone, false)?;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(Event {
        id: uuid::Uuid::new_v4().to_string(),
        calendar_id: calendar_id.to_string(),
        project_id: None,
        title: event
            .summary
            .clone()
            .unwrap_or_else(|| "(no title)".to_string()),
        description: event.description.clone(),
        location: event.location.clone(),
        start_at,
        end_at,
        all_day,
        rrule: event.recurrence.as_ref().and_then(|rules| {
            rules
                .iter()
                .find(|rule| rule.starts_with("RRULE:"))
                .cloned()
        }),
        google_id: Some(google_id),
        google_etag: event.etag.clone(),
        reminder_minutes: None,
        timezone,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    })
}

pub fn is_cancelled(event: &GoogleEvent) -> bool {
    event.status.as_deref() == Some("cancelled")
}

pub fn is_recurring_exception(event: &GoogleEvent) -> bool {
    event.recurring_event_id.is_some() && event.original_start_time.is_some()
}

pub fn remote_updated_at(event: &GoogleEvent) -> Option<String> {
    event.updated.as_deref().and_then(|updated| {
        chrono::DateTime::parse_from_rfc3339(updated)
            .ok()
            .map(|updated| {
                updated
                    .with_timezone(&chrono::Utc)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
    })
}

pub fn local_event_insert_body(event: &Event) -> Result<serde_json::Value> {
    Ok(json!({
        "summary": event.title,
        "description": event.description,
        "location": event.location,
        "start": local_event_time_body(event, true)?,
        "end": local_event_time_body(event, false)?,
        "recurrence": event.rrule.as_ref().map(|rule| vec![rule.clone()]),
    }))
}

pub fn local_event_patch_body(event: &Event) -> Result<serde_json::Value> {
    local_event_insert_body(event)
}

fn local_event_time_body(event: &Event, is_start: bool) -> Result<serde_json::Value> {
    let timestamp = if is_start {
        &event.start_at
    } else {
        &event.end_at
    };

    if event.all_day {
        let date = &timestamp[..10];
        if is_start {
            return Ok(json!({ "date": date }));
        }

        let end_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| anyhow!("invalid all-day end date '{}'", date))?
            .succ_opt()
            .ok_or_else(|| anyhow!("invalid all-day end date '{}'", date))?;
        return Ok(json!({ "date": end_date.format("%Y-%m-%d").to_string() }));
    }

    let local = crate::time::resolve_local_datetime(timestamp, &event.timezone)?;
    Ok(json!({
        "dateTime": local.to_rfc3339(),
        "timeZone": event.timezone,
    }))
}

fn map_google_time(
    value: Option<&GoogleEventTime>,
    fallback_timezone: &str,
    is_start: bool,
) -> Result<(String, bool)> {
    let value = value.ok_or_else(|| anyhow!("google event missing time value"))?;
    if let Some(date_time) = value.date_time.as_deref() {
        let timezone = value.time_zone.as_deref().unwrap_or(fallback_timezone);
        let parsed = chrono::DateTime::parse_from_rfc3339(date_time)
            .map_err(|_| anyhow!("invalid google event datetime '{}'", date_time))?;
        let local = parsed.with_timezone(
            &crate::time::parse_timezone(timezone).map_err(|err| anyhow!(err.to_string()))?,
        );
        return Ok((local.format(crate::time::STORAGE_FORMAT).to_string(), false));
    }

    let date = value
        .date
        .as_deref()
        .ok_or_else(|| anyhow!("google event missing date"))?;
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| anyhow!("invalid google event date '{}'", date))?;
    let timestamp = if is_start {
        format!("{} 00:00:00", parsed.format("%Y-%m-%d"))
    } else {
        let inclusive_end = parsed
            .pred_opt()
            .ok_or_else(|| anyhow!("invalid all-day end date '{}'", date))?;
        format!("{} 23:59:59", inclusive_end.format("%Y-%m-%d"))
    };
    Ok((timestamp, true))
}

#[cfg(test)]
mod tests {
    use super::{
        google_event_to_local, is_recurring_exception, local_event_insert_body, GoogleEvent,
    };
    use crate::models::Event;

    #[test]
    fn maps_google_event_datetime_into_wall_clock_timezone() {
        let event = serde_json::from_value::<GoogleEvent>(serde_json::json!({
            "id": "abc",
            "etag": "\"etag\"",
            "summary": "Planning",
            "start": {
                "dateTime": "2026-04-12T09:00:00+02:00",
                "timeZone": "Europe/Rome"
            },
            "end": {
                "dateTime": "2026-04-12T10:00:00+02:00",
                "timeZone": "Europe/Rome"
            }
        }))
        .unwrap();

        let local = google_event_to_local("calendar-1", &event).unwrap();
        assert_eq!(local.start_at, "2026-04-12 09:00:00");
        assert_eq!(local.end_at, "2026-04-12 10:00:00");
        assert_eq!(local.timezone, "Europe/Rome");
    }

    #[test]
    fn skips_recurring_exceptions_for_first_pass_support() {
        let event = serde_json::from_value::<GoogleEvent>(serde_json::json!({
            "id": "abc",
            "recurringEventId": "master",
            "originalStartTime": {
                "dateTime": "2026-04-12T09:00:00+02:00"
            }
        }))
        .unwrap();

        assert!(is_recurring_exception(&event));
    }

    #[test]
    fn builds_google_insert_body_for_all_day_event() {
        let mut event = Event::new(
            "calendar-1",
            "Offsite",
            "2026-04-12 00:00:00",
            "2026-04-12 23:59:59",
            "UTC",
        );
        event.all_day = true;

        let body = local_event_insert_body(&event).unwrap();
        assert_eq!(body["start"]["date"], "2026-04-12");
        assert_eq!(body["end"]["date"], "2026-04-13");
    }
}
