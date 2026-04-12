use std::collections::HashSet;

use anyhow::Error as AnyError;
use rusqlite::{Connection, Error as SqliteError, ErrorCode};
use uuid::Uuid;

use crate::{
    db,
    google::discovery::DiscoveredGoogleCalendar,
    models::{Calendar, CalendarSource},
    sync::state::CalendarSyncState,
};

#[derive(Debug, Clone)]
pub struct CreateCalendarInput {
    pub name: String,
    pub color: String,
    pub source: CalendarSource,
    pub google_id: Option<String>,
    pub visible: bool,
    pub position: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UpdateCalendarInput {
    pub id: String,
    pub name: Option<String>,
    pub color: Option<String>,
    pub source: Option<CalendarSource>,
    pub google_id: Option<Option<String>>,
    pub visible: Option<bool>,
    pub position: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarServiceError {
    NotFound { resource: &'static str, id: String },
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl std::fmt::Display for CalendarServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { resource, id } => write!(f, "{} '{}' not found", resource, id),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for CalendarServiceError {}

pub fn create_calendar(
    conn: &Connection,
    input: CreateCalendarInput,
) -> Result<Calendar, CalendarServiceError> {
    let source = input.source;
    let google_id = normalize_optional(input.google_id);
    validate_calendar_source(&source, google_id.as_deref())?;
    if let Some(google_id) = google_id.as_deref() {
        ensure_google_calendar_not_imported(conn, google_id, None)?;
    }
    let position = match input.position {
        Some(position) => position,
        None => db::next_calendar_position(conn).map_err(map_internal)?,
    };

    let now = timestamp_now();
    let calendar = Calendar {
        id: Uuid::new_v4().to_string(),
        name: normalize_required(input.name, "name")?,
        color: normalize_required(input.color, "color")?,
        source,
        google_id,
        visible: input.visible,
        position,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };

    db::insert_calendar(conn, &calendar)
        .map_err(|err| map_write_error(err, calendar.google_id.as_deref()))?;
    Ok(calendar)
}

pub fn update_calendar(
    conn: &Connection,
    input: UpdateCalendarInput,
) -> Result<Calendar, CalendarServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    let mut calendar = db::get_calendar(&tx, &input.id)
        .map_err(map_internal)?
        .ok_or_else(|| CalendarServiceError::NotFound {
            resource: "calendar",
            id: input.id.clone(),
        })?;
    let original_source = calendar.source.clone();
    let original_google_id = calendar.google_id.clone();

    if let Some(name) = input.name {
        calendar.name = normalize_required(name, "name")?;
    }
    if let Some(color) = input.color {
        calendar.color = normalize_required(color, "color")?;
    }
    if let Some(source) = input.source {
        calendar.source = source;
    }
    if let Some(visible) = input.visible {
        calendar.visible = visible;
    }
    if let Some(position) = input.position {
        calendar.position = position;
    }
    if let Some(google_id) = input.google_id {
        calendar.google_id = normalize_optional(google_id);
    }
    if calendar.source == CalendarSource::Local {
        calendar.google_id = None;
    }

    if original_source == CalendarSource::Google
        && calendar.source == CalendarSource::Google
        && original_google_id != calendar.google_id
    {
        return Err(CalendarServiceError::Validation(
            "changing google_id on an existing google calendar is not supported".to_string(),
        ));
    }

    validate_calendar_source(&calendar.source, calendar.google_id.as_deref())?;
    if let Some(google_id) = calendar.google_id.as_deref() {
        ensure_google_calendar_not_imported(&tx, google_id, Some(&calendar.id))?;
    }

    db::update_calendar(&tx, &calendar)
        .map_err(|err| map_write_error(err, calendar.google_id.as_deref()))?;
    if original_source == CalendarSource::Google && calendar.source == CalendarSource::Local {
        db::detach_google_sync_state_for_calendar(&tx, &calendar.id).map_err(map_internal)?;
    }

    let updated = db::get_calendar(&tx, &calendar.id)
        .map_err(map_internal)?
        .ok_or_else(|| {
            CalendarServiceError::Internal("calendar disappeared after update".into())
        })?;
    tx.commit().map_err(map_internal)?;
    Ok(updated)
}

pub fn import_google_calendar(
    conn: &Connection,
    calendar: &DiscoveredGoogleCalendar,
    position: Option<i64>,
) -> Result<Calendar, CalendarServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    let imported = create_calendar(
        &tx,
        CreateCalendarInput {
            name: calendar.name.clone(),
            color: calendar.color.clone(),
            source: CalendarSource::Google,
            google_id: Some(calendar.google_id.clone()),
            visible: true,
            position,
        },
    )?;
    crate::sync::state::upsert_calendar_sync_state(
        &tx,
        &CalendarSyncState {
            calendar_id: imported.id.clone(),
            google_access_role: calendar.access_role.clone(),
            writable: calendar.writable,
            last_synced_at: None,
            last_sync_error_code: None,
            last_sync_error_message: None,
            created_at: String::new(),
            updated_at: String::new(),
        },
    )
    .map_err(map_internal)?;
    tx.commit().map_err(map_internal)?;
    Ok(imported)
}

pub fn filter_unimported_google_calendars(
    conn: &Connection,
    discovered: Vec<DiscoveredGoogleCalendar>,
) -> Result<Vec<DiscoveredGoogleCalendar>, CalendarServiceError> {
    let imported_google_ids = db::load_calendars(conn)
        .map_err(map_internal)?
        .into_iter()
        .filter(|calendar| calendar.source == CalendarSource::Google)
        .filter_map(|calendar| calendar.google_id)
        .collect::<HashSet<_>>();

    Ok(discovered
        .into_iter()
        .filter(|calendar| !imported_google_ids.contains(&calendar.google_id))
        .collect())
}

fn normalize_required(
    value: impl Into<String>,
    field: &'static str,
) -> Result<String, CalendarServiceError> {
    let trimmed = value.into().trim().to_string();
    if trimmed.is_empty() {
        return Err(CalendarServiceError::Validation(format!(
            "{} cannot be empty",
            field
        )));
    }
    Ok(trimmed)
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

fn validate_calendar_source(
    source: &CalendarSource,
    google_id: Option<&str>,
) -> Result<(), CalendarServiceError> {
    match source {
        CalendarSource::Google if google_id.is_none() => Err(CalendarServiceError::Validation(
            "google calendars require a google_id".to_string(),
        )),
        CalendarSource::Local if google_id.is_some() => Err(CalendarServiceError::Validation(
            "local calendars cannot set a google_id".to_string(),
        )),
        _ => Ok(()),
    }
}

fn ensure_google_calendar_not_imported(
    conn: &Connection,
    google_id: &str,
    current_calendar_id: Option<&str>,
) -> Result<(), CalendarServiceError> {
    let existing = db::get_google_calendar_by_google_id(conn, google_id).map_err(map_internal)?;
    if let Some(existing) = existing {
        if current_calendar_id != Some(existing.id.as_str()) {
            return Err(CalendarServiceError::Conflict(format!(
                "google calendar '{}' is already imported",
                google_id
            )));
        }
    }
    Ok(())
}

fn map_write_error(err: AnyError, google_id: Option<&str>) -> CalendarServiceError {
    if is_google_id_unique_violation(&err) {
        if let Some(google_id) = google_id {
            return CalendarServiceError::Conflict(format!(
                "google calendar '{}' is already imported",
                google_id
            ));
        }
        return CalendarServiceError::Conflict("google calendar is already imported".to_string());
    }
    map_internal(err)
}

fn is_google_id_unique_violation(err: &AnyError) -> bool {
    let Some(sqlite_err) = err.downcast_ref::<SqliteError>() else {
        return false;
    };

    match sqlite_err {
        SqliteError::SqliteFailure(code, message)
            if code.code == ErrorCode::ConstraintViolation =>
        {
            message
                .as_deref()
                .map(|message| {
                    message.contains("idx_calendars_google_id_unique")
                        || message.contains("calendars.google_id")
                })
                .unwrap_or(true)
        }
        _ => false,
    }
}

fn map_internal(err: impl std::fmt::Display) -> CalendarServiceError {
    CalendarServiceError::Internal(err.to_string())
}

fn timestamp_now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");
        let conn = db::open_at(&db_path).unwrap();
        (temp, conn)
    }

    #[test]
    fn create_google_calendar_requires_google_id() {
        let (_temp, conn) = open_test_db();
        let err = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: None,
                visible: true,
                position: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            CalendarServiceError::Validation("google calendars require a google_id".to_string())
        );
    }

    #[test]
    fn import_google_calendar_rejects_duplicate_google_id() {
        let (_temp, conn) = open_test_db();
        let discovered = DiscoveredGoogleCalendar {
            google_id: "work@example.com".to_string(),
            name: "Work".to_string(),
            color: "#50f872".to_string(),
            primary: false,
            access_role: Some("writer".to_string()),
            writable: true,
        };

        import_google_calendar(&conn, &discovered, None).unwrap();
        let err = import_google_calendar(&conn, &discovered, None).unwrap_err();
        assert_eq!(
            err,
            CalendarServiceError::Conflict(
                "google calendar 'work@example.com' is already imported".to_string()
            )
        );
    }

    #[test]
    fn import_google_calendar_persists_access_role_and_writability() {
        let (_temp, conn) = open_test_db();
        let imported = import_google_calendar(
            &conn,
            &DiscoveredGoogleCalendar {
                google_id: "readonly@example.com".to_string(),
                name: "Readonly".to_string(),
                color: "#123456".to_string(),
                primary: false,
                access_role: Some("reader".to_string()),
                writable: false,
            },
            None,
        )
        .unwrap();

        let sync_state = crate::sync::state::load_calendar_sync_state(&conn, &imported.id)
            .unwrap()
            .unwrap();
        assert_eq!(sync_state.google_access_role.as_deref(), Some("reader"));
        assert!(!sync_state.writable);
    }

    #[test]
    fn update_calendar_to_local_clears_google_id() {
        let (_temp, conn) = open_test_db();
        let created = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: Some(1),
            },
        )
        .unwrap();

        let updated = update_calendar(
            &conn,
            UpdateCalendarInput {
                id: created.id,
                name: Some("Personal".to_string()),
                color: None,
                source: Some(CalendarSource::Local),
                google_id: None,
                visible: None,
                position: None,
            },
        )
        .unwrap();

        assert_eq!(updated.source, CalendarSource::Local);
        assert_eq!(updated.google_id, None);
        assert_eq!(updated.name, "Personal");
    }

    #[test]
    fn update_calendar_to_local_detaches_event_sync_state() {
        let (_temp, conn) = open_test_db();
        let created = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: Some(1),
            },
        )
        .unwrap();
        let now = timestamp_now();
        db::insert_event(
            &conn,
            &crate::models::Event {
                id: uuid::Uuid::new_v4().to_string(),
                calendar_id: created.id.clone(),
                project_id: None,
                title: "Imported".to_string(),
                description: None,
                location: None,
                start_at: "2026-04-06 10:00:00".to_string(),
                end_at: "2026-04-06 11:00:00".to_string(),
                all_day: false,
                rrule: None,
                google_id: Some("google-event-1".to_string()),
                google_etag: Some("\"etag-1\"".to_string()),
                reminder_minutes: None,
                timezone: "UTC".to_string(),
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            },
        )
        .unwrap();
        db::upsert_sync_token(&conn, &created.id, "sync-token-1").unwrap();

        let updated = update_calendar(
            &conn,
            UpdateCalendarInput {
                id: created.id.clone(),
                name: None,
                color: None,
                source: Some(CalendarSource::Local),
                google_id: None,
                visible: None,
                position: None,
            },
        )
        .unwrap();

        let event = db::load_events(&conn)
            .unwrap()
            .into_iter()
            .find(|event| event.calendar_id == created.id)
            .unwrap();
        assert_eq!(updated.source, CalendarSource::Local);
        assert_eq!(updated.google_id, None);
        assert_eq!(event.google_id, None);
        assert_eq!(event.google_etag, None);
        assert_eq!(db::get_sync_token(&conn, &created.id).unwrap(), None);
    }

    #[test]
    fn update_calendar_rejects_google_id_changes_for_existing_google_calendar() {
        let (_temp, conn) = open_test_db();
        let created = create_calendar(
            &conn,
            CreateCalendarInput {
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: Some(1),
            },
        )
        .unwrap();

        let err = update_calendar(
            &conn,
            UpdateCalendarInput {
                id: created.id,
                name: None,
                color: None,
                source: Some(CalendarSource::Google),
                google_id: Some(Some("personal@example.com".to_string())),
                visible: None,
                position: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            CalendarServiceError::Validation(
                "changing google_id on an existing google calendar is not supported".to_string()
            )
        );
    }

    #[test]
    fn filter_unimported_google_calendars_excludes_existing_rows() {
        let (_temp, conn) = open_test_db();
        import_google_calendar(
            &conn,
            &DiscoveredGoogleCalendar {
                google_id: "work@example.com".to_string(),
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                primary: false,
                access_role: Some("writer".to_string()),
                writable: true,
            },
            None,
        )
        .unwrap();

        let filtered = filter_unimported_google_calendars(
            &conn,
            vec![
                DiscoveredGoogleCalendar {
                    google_id: "work@example.com".to_string(),
                    name: "Work".to_string(),
                    color: "#50f872".to_string(),
                    primary: false,
                    access_role: Some("writer".to_string()),
                    writable: true,
                },
                DiscoveredGoogleCalendar {
                    google_id: "personal@example.com".to_string(),
                    name: "Personal".to_string(),
                    color: "#ffaa00".to_string(),
                    primary: true,
                    access_role: Some("reader".to_string()),
                    writable: false,
                },
            ],
        )
        .unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].google_id, "personal@example.com");
    }
}
