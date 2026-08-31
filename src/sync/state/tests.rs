use rusqlite::Connection;
use tempfile::TempDir;

use super::{
    get_outbox_entry, insert_conflict, load_calendar_sync_state, load_event_sync_state,
    upsert_calendar_sync_state, CalendarSyncState, ConflictResolutionStatus, OutboxOperation,
    SyncConflict, SyncState, GOOGLE_PROVIDER,
};
use crate::{
    db,
    models::{CalendarSource, Event},
};

fn open_test_db() -> (TempDir, Connection) {
    let temp = TempDir::new().unwrap();
    let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
    (temp, conn)
}

#[test]
fn repeated_google_updates_coalesce_to_one_create_operation() {
    let (_temp, conn) = open_test_db();
    let calendar = crate::calendar_service::create_calendar(
        &conn,
        crate::calendar_service::CreateCalendarInput {
            name: "Google".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("primary@example.com".to_string()),
            visible: true,
            position: None,
        },
    )
    .unwrap();

    let event = crate::event_service::save_event(
        &conn,
        Event::new(
            calendar.id.clone(),
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        ),
        true,
    )
    .unwrap();

    let outbox = get_outbox_entry(&conn, GOOGLE_PROVIDER, &event.id)
        .unwrap()
        .unwrap();
    assert_eq!(outbox.operation, OutboxOperation::Create);

    let sync_state = load_event_sync_state(&conn, &event.id).unwrap().unwrap();
    assert_eq!(sync_state.sync_state, SyncState::PendingCreate);
}

#[test]
fn read_only_google_calendar_blocks_local_changes() {
    let (_temp, conn) = open_test_db();
    let calendar = crate::calendar_service::create_calendar(
        &conn,
        crate::calendar_service::CreateCalendarInput {
            name: "Read Only".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("readonly@example.com".to_string()),
            visible: true,
            position: None,
        },
    )
    .unwrap();
    upsert_calendar_sync_state(
        &conn,
        &CalendarSyncState {
            calendar_id: calendar.id.clone(),
            google_access_role: Some("reader".to_string()),
            writable: false,
            last_synced_at: None,
            last_sync_error_code: None,
            last_sync_error_message: None,
            pending_outbox: 0,
            pending_conflicts: 0,
            created_at: String::new(),
            updated_at: String::new(),
        },
    )
    .unwrap();

    let err = crate::event_service::save_event(
        &conn,
        Event::new(
            calendar.id.clone(),
            "Blocked",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        ),
        true,
    )
    .unwrap_err();
    assert!(err.to_string().contains("read-only"));
}

#[test]
fn delete_supersedes_pending_create_when_no_remote_event_exists() {
    let (_temp, conn) = open_test_db();
    let calendar = crate::calendar_service::create_calendar(
        &conn,
        crate::calendar_service::CreateCalendarInput {
            name: "Google".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("primary@example.com".to_string()),
            visible: true,
            position: None,
        },
    )
    .unwrap();

    let event = crate::event_service::save_event(
        &conn,
        Event::new(
            calendar.id.clone(),
            "Draft",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        ),
        true,
    )
    .unwrap();
    crate::event_service::delete_event(&conn, &event.id).unwrap();

    assert!(get_outbox_entry(&conn, GOOGLE_PROVIDER, &event.id)
        .unwrap()
        .is_none());
    assert!(load_event_sync_state(&conn, &event.id).unwrap().is_none());
}

#[test]
fn calendar_sync_state_reports_pending_counts() {
    let (_temp, conn) = open_test_db();
    let calendar = crate::calendar_service::create_calendar(
        &conn,
        crate::calendar_service::CreateCalendarInput {
            name: "Google".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("primary@example.com".to_string()),
            visible: true,
            position: None,
        },
    )
    .unwrap();
    upsert_calendar_sync_state(
        &conn,
        &CalendarSyncState {
            calendar_id: calendar.id.clone(),
            google_access_role: Some("owner".to_string()),
            writable: true,
            last_synced_at: None,
            last_sync_error_code: None,
            last_sync_error_message: None,
            pending_outbox: 0,
            pending_conflicts: 0,
            created_at: String::new(),
            updated_at: String::new(),
        },
    )
    .unwrap();
    let event = crate::event_service::save_event(
        &conn,
        Event::new(
            calendar.id.clone(),
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        ),
        true,
    )
    .unwrap();
    insert_conflict(
        &conn,
        &SyncConflict {
            id: String::new(),
            event_id: event.id,
            calendar_id: calendar.id.clone(),
            local_snapshot: "{}".to_string(),
            remote_snapshot: "{}".to_string(),
            remote_etag: Some("\"etag\"".to_string()),
            detected_at: String::new(),
            resolution_status: ConflictResolutionStatus::Pending,
            resolution_strategy: None,
            resolved_at: None,
        },
    )
    .unwrap();

    let state = load_calendar_sync_state(&conn, &calendar.id)
        .unwrap()
        .unwrap();
    assert_eq!(state.pending_outbox, 1);
    assert_eq!(state.pending_conflicts, 1);
}
