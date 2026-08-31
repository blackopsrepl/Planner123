use anyhow::Error as AnyError;
use rusqlite::Connection;
use uuid::Uuid;

use crate::{
    db,
    models::{CalendarSource, Event},
    sync::state,
    time,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventServiceError {
    NotFound { resource: &'static str, id: String },
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl std::fmt::Display for EventServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { resource, id } => write!(f, "{} '{}' not found", resource, id),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for EventServiceError {}

pub fn save_event(
    conn: &Connection,
    event: Event,
    is_new: bool,
) -> Result<Event, EventServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    let saved = save_event_in_transaction(&tx, event, is_new)?;
    tx.commit().map_err(map_internal)?;
    Ok(saved)
}

pub(crate) fn save_event_in_transaction(
    conn: &Connection,
    mut event: Event,
    is_new: bool,
) -> Result<Event, EventServiceError> {
    let previous_event = if is_new {
        None
    } else {
        Some(
            db::get_event(conn, &event.id)
                .map_err(map_internal)?
                .ok_or_else(|| EventServiceError::NotFound {
                    resource: "event",
                    id: event.id.clone(),
                })?,
        )
    };
    let calendar = prepare_event(conn, &mut event, is_new)?;

    if is_new {
        db::insert_event(conn, &event).map_err(map_internal)?;
    } else {
        db::update_event(conn, &event).map_err(map_internal)?;
    }
    state::record_local_event_save(conn, &calendar, previous_event.as_ref(), &event)
        .map_err(map_sync_error)?;

    db::get_event(conn, &event.id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::Internal("event disappeared after save".to_string()))
}

pub fn delete_event(conn: &Connection, event_id: &str) -> Result<(), EventServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    delete_event_in_transaction(&tx, event_id)?;
    tx.commit().map_err(map_internal)?;
    Ok(())
}

pub(crate) fn delete_event_in_transaction(
    conn: &Connection,
    event_id: &str,
) -> Result<(), EventServiceError> {
    let event = db::get_event(conn, event_id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::NotFound {
            resource: "event",
            id: event_id.to_string(),
        })?;
    let calendar = db::get_calendar(conn, &event.calendar_id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::NotFound {
            resource: "calendar",
            id: event.calendar_id.clone(),
        })?;
    if calendar.source == CalendarSource::Google
        && !state::google_calendar_writable(conn, &calendar).map_err(map_sync_error)?
    {
        return Err(EventServiceError::Conflict(format!(
            "calendar '{}' is read-only and cannot accept local edits",
            calendar.name
        )));
    }
    state::record_local_event_delete(conn, &calendar, &event).map_err(map_sync_error)?;
    db::soft_delete_event(conn, event_id).map_err(map_internal)?;
    Ok(())
}

fn prepare_event(
    conn: &Connection,
    event: &mut Event,
    is_new: bool,
) -> Result<crate::models::Calendar, EventServiceError> {
    if is_new {
        event.id = if event.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            event.id.trim().to_string()
        };
    } else {
        ensure_event_exists(conn, &event.id)?;
    }

    let calendar = db::get_calendar(conn, &event.calendar_id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::NotFound {
            resource: "calendar",
            id: event.calendar_id.clone(),
        })?;

    if let Some(project_id) = event.project_id.as_deref() {
        let exists = db::get_project(conn, project_id).map_err(map_internal)?;
        if exists.is_none() {
            return Err(EventServiceError::NotFound {
                resource: "project",
                id: project_id.to_string(),
            });
        }
    }

    if event.title.trim().is_empty() {
        return Err(EventServiceError::Validation(
            "title cannot be empty".to_string(),
        ));
    }

    event.title = event.title.trim().to_string();
    event.description = normalize_optional(event.description.take());
    event.location = normalize_optional(event.location.take());
    event.rrule = normalize_rrule(normalize_optional(event.rrule.take()))?;
    event.timezone = time::normalize_timezone(event.timezone.trim()).map_err(map_validation)?;

    if event.all_day {
        let (start_at, end_at) = time::normalize_all_day_bounds(&event.start_at, &event.end_at)
            .map_err(map_validation)?;
        event.start_at = start_at;
        event.end_at = end_at;
    } else {
        event.start_at = time::normalize_timestamp(&event.start_at).map_err(map_validation)?;
        event.end_at = time::normalize_timestamp(&event.end_at).map_err(map_validation)?;
    }

    time::validate_range(&event.start_at, &event.end_at, &event.timezone)
        .map_err(map_validation)?;

    if calendar.source == CalendarSource::Google
        && !state::google_calendar_writable(conn, &calendar).map_err(map_sync_error)?
    {
        return Err(EventServiceError::Conflict(format!(
            "calendar '{}' is read-only and cannot accept local edits",
            calendar.name
        )));
    }

    let now = timestamp_now();
    if is_new {
        event.created_at = now.clone();
    }
    event.updated_at = now;

    Ok(calendar)
}

fn normalize_rrule(value: Option<String>) -> Result<Option<String>, EventServiceError> {
    let Some(value) = value else {
        return Ok(None);
    };

    if value.starts_with("RRULE:") {
        return Err(EventServiceError::Validation(
            "rrule must contain rule content beginning with FREQ=, not an RRULE: property prefix"
                .to_string(),
        ));
    }
    if !value.starts_with("FREQ=") {
        return Err(EventServiceError::Validation(
            "rrule must contain rule content beginning with FREQ=".to_string(),
        ));
    }

    Ok(Some(value))
}

fn ensure_event_exists(conn: &Connection, event_id: &str) -> Result<(), EventServiceError> {
    if db::get_event(conn, event_id)
        .map_err(map_internal)?
        .is_none()
    {
        return Err(EventServiceError::NotFound {
            resource: "event",
            id: event_id.to_string(),
        });
    }
    Ok(())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn map_internal(err: impl std::fmt::Display) -> EventServiceError {
    EventServiceError::Internal(err.to_string())
}

fn map_validation(err: AnyError) -> EventServiceError {
    EventServiceError::Validation(err.to_string())
}

fn map_sync_error(err: impl std::fmt::Display) -> EventServiceError {
    let message = err.to_string();
    if message.contains("read-only") || message.contains("not supported") {
        EventServiceError::Conflict(message)
    } else {
        EventServiceError::Internal(message)
    }
}

fn timestamp_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{delete_event, save_event, EventServiceError};
    use crate::{db, models::Event};

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
        (temp, conn)
    }

    fn first_calendar_id(conn: &Connection) -> String {
        db::load_calendars(conn).unwrap()[0].id.clone()
    }

    #[test]
    fn save_event_keeps_wall_clock_time_with_timezone() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let event = Event::new(
            calendar_id,
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "Europe/Rome",
        );

        let saved = save_event(&conn, event, true).unwrap();
        assert_eq!(saved.timezone, "Europe/Rome");
        assert_eq!(saved.start_at, "2026-04-12 09:00:00");
        assert_eq!(saved.end_at, "2026-04-12 10:00:00");
    }

    #[test]
    fn save_event_normalizes_all_day_bounds() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let mut event = Event::new(
            calendar_id,
            "Offsite",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.all_day = true;

        let saved = save_event(&conn, event, true).unwrap();
        assert_eq!(saved.start_at, "2026-04-12 00:00:00");
        assert_eq!(saved.end_at, "2026-04-12 23:59:59");
    }

    #[test]
    fn save_event_requires_unprefixed_rrule_content() {
        let (_temp, conn) = open_test_db();
        let calendar_id = first_calendar_id(&conn);
        let mut event = Event::new(
            calendar_id,
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.rrule = Some("RRULE:FREQ=WEEKLY".to_string());

        let err = save_event(&conn, event, true).unwrap_err();
        assert!(
            matches!(err, EventServiceError::Validation(message) if message.contains("RRULE:"))
        );
    }

    #[test]
    fn delete_event_requires_existing_row() {
        let (_temp, conn) = open_test_db();
        let err = delete_event(&conn, "missing").unwrap_err();
        assert_eq!(
            err,
            EventServiceError::NotFound {
                resource: "event",
                id: "missing".to_string()
            }
        );
    }
}
