use super::super::*;
use crate::models::{Calendar, CalendarSource, Event};
use tempfile::TempDir;

#[test]
fn google_calendar_unique_index_blocks_duplicate_active_rows() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");
    let conn = open_at(&db_path).unwrap();
    let now = "2026-04-06 10:00:00".to_string();

    insert_calendar(
        &conn,
        &Calendar {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Work".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("work@example.com".to_string()),
            visible: true,
            position: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        },
    )
    .unwrap();

    let err = insert_calendar(
        &conn,
        &Calendar {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Work Copy".to_string(),
            color: "#ffaa00".to_string(),
            source: CalendarSource::Google,
            google_id: Some("work@example.com".to_string()),
            visible: true,
            position: 2,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        },
    )
    .unwrap_err();

    let sqlite_err = err.downcast_ref::<rusqlite::Error>().unwrap();
    match sqlite_err {
        rusqlite::Error::SqliteFailure(code, _) => {
            assert_eq!(code.code, rusqlite::ErrorCode::ConstraintViolation);
        }
        other => panic!("expected sqlite constraint violation, got {other:?}"),
    }
}

#[test]
fn event_google_id_unique_index_is_scoped_to_calendar() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");
    let conn = open_at(&db_path).unwrap();
    let now = "2026-04-06 10:00:00".to_string();
    let first_calendar = Calendar {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Work".to_string(),
        color: "#50f872".to_string(),
        source: CalendarSource::Google,
        google_id: Some("work@example.com".to_string()),
        visible: true,
        position: 1,
        created_at: now.clone(),
        updated_at: now.clone(),
        deleted_at: None,
    };
    let second_calendar = Calendar {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Personal".to_string(),
        color: "#ffaa00".to_string(),
        source: CalendarSource::Google,
        google_id: Some("personal@example.com".to_string()),
        visible: true,
        position: 2,
        created_at: now.clone(),
        updated_at: now.clone(),
        deleted_at: None,
    };
    insert_calendar(&conn, &first_calendar).unwrap();
    insert_calendar(&conn, &second_calendar).unwrap();

    let first_event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        calendar_id: first_calendar.id.clone(),
        project_id: None,
        title: "Sync target".to_string(),
        description: None,
        location: None,
        start_at: "2026-04-06 10:00:00".to_string(),
        end_at: "2026-04-06 11:00:00".to_string(),
        all_day: false,
        rrule: None,
        google_id: Some("shared-google-event".to_string()),
        google_etag: Some("\"etag-1\"".to_string()),
        reminder_minutes: None,
        timezone: "UTC".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        deleted_at: None,
    };
    insert_event(&conn, &first_event).unwrap();

    let duplicate_same_calendar = insert_event(
        &conn,
        &Event {
            id: uuid::Uuid::new_v4().to_string(),
            calendar_id: first_calendar.id.clone(),
            project_id: None,
            title: "Duplicate".to_string(),
            description: None,
            location: None,
            start_at: "2026-04-06 12:00:00".to_string(),
            end_at: "2026-04-06 13:00:00".to_string(),
            all_day: false,
            rrule: None,
            google_id: Some("shared-google-event".to_string()),
            google_etag: Some("\"etag-2\"".to_string()),
            reminder_minutes: None,
            timezone: "UTC".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        },
    )
    .unwrap_err();
    let sqlite_err = duplicate_same_calendar
        .downcast_ref::<rusqlite::Error>()
        .unwrap();
    match sqlite_err {
        rusqlite::Error::SqliteFailure(code, _) => {
            assert_eq!(code.code, rusqlite::ErrorCode::ConstraintViolation);
        }
        other => panic!("expected sqlite constraint violation, got {other:?}"),
    }

    insert_event(
        &conn,
        &Event {
            id: uuid::Uuid::new_v4().to_string(),
            calendar_id: second_calendar.id.clone(),
            project_id: None,
            title: "Allowed in other calendar".to_string(),
            description: None,
            location: None,
            start_at: "2026-04-06 12:00:00".to_string(),
            end_at: "2026-04-06 13:00:00".to_string(),
            all_day: false,
            rrule: None,
            google_id: Some("shared-google-event".to_string()),
            google_etag: Some("\"etag-3\"".to_string()),
            reminder_minutes: None,
            timezone: "UTC".to_string(),
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        },
    )
    .unwrap();
}
