use super::super::*;
use crate::models::{Calendar, CalendarSource, Event};
use tempfile::TempDir;

#[test]
fn detach_google_sync_state_for_calendar_clears_event_sync_fields_and_token() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");
    let conn = open_at(&db_path).unwrap();
    let now = "2026-04-06 10:00:00".to_string();
    let calendar = Calendar {
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
    insert_calendar(&conn, &calendar).unwrap();
    insert_event(
        &conn,
        &Event {
            id: uuid::Uuid::new_v4().to_string(),
            calendar_id: calendar.id.clone(),
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
            updated_at: now.clone(),
            deleted_at: None,
        },
    )
    .unwrap();
    upsert_sync_token(&conn, &calendar.id, "sync-token-1").unwrap();

    detach_google_sync_state_for_calendar(&conn, &calendar.id).unwrap();

    let detached_event = load_events(&conn)
        .unwrap()
        .into_iter()
        .find(|event| event.calendar_id == calendar.id)
        .unwrap();
    assert_eq!(detached_event.google_id, None);
    assert_eq!(detached_event.google_etag, None);
    assert_eq!(get_sync_token(&conn, &calendar.id).unwrap(), None);
}
