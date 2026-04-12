use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::{
    db,
    google::types::{is_cancelled, remote_updated_at, GoogleEvent},
    sync::{
        pull,
        state::{
            self, ConflictResolutionStrategy, EventSyncState, OutboxOperation, SyncState,
            GOOGLE_PROVIDER,
        },
    },
};

pub fn list_pending_conflicts(conn: &Connection) -> Result<Vec<state::SyncConflict>> {
    state::load_conflicts(conn, true)
}

pub fn resolve_conflict(
    conn: &Connection,
    conflict_id: &str,
    strategy: ConflictResolutionStrategy,
) -> Result<state::SyncConflict> {
    let tx = conn.unchecked_transaction()?;
    let conflict = state::get_conflict(&tx, conflict_id)?
        .ok_or_else(|| anyhow!("conflict '{}' not found", conflict_id))?;
    let remote_event = serde_json::from_str::<GoogleEvent>(&conflict.remote_snapshot)?;
    let existing_event = db::get_event_including_deleted(&tx, &conflict.event_id)?
        .ok_or_else(|| anyhow!("event '{}' not found", conflict.event_id))?;
    let existing_state = state::load_event_sync_state(&tx, &existing_event.id)?;

    match strategy {
        ConflictResolutionStrategy::KeepLocal => {
            let operation = if existing_event.deleted_at.is_some() {
                OutboxOperation::Delete
            } else {
                OutboxOperation::Update
            };
            let sync_state = match operation {
                OutboxOperation::Delete => SyncState::PendingDelete,
                OutboxOperation::Update => SyncState::PendingUpdate,
                OutboxOperation::Create => SyncState::PendingCreate,
            };
            db::update_event_google_version(
                &tx,
                &existing_event.id,
                remote_event
                    .id
                    .as_deref()
                    .or(existing_event.google_id.as_deref()),
                remote_event
                    .etag
                    .as_deref()
                    .or(existing_event.google_etag.as_deref()),
            )?;
            state::upsert_event_sync_state(
                &tx,
                &EventSyncState {
                    event_id: existing_event.id.clone(),
                    sync_state,
                    last_synced_at: existing_state
                        .as_ref()
                        .and_then(|state| state.last_synced_at.clone()),
                    last_push_attempt_at: None,
                    last_remote_modified_at: remote_updated_at(&remote_event),
                    last_sync_error_code: None,
                    last_sync_error_message: None,
                    remote_payload: Some(serde_json::to_string(&remote_event)?),
                    created_at: existing_state
                        .map(|state| state.created_at)
                        .unwrap_or_default(),
                    updated_at: String::new(),
                },
            )?;
            state::upsert_outbox_entry(
                &tx,
                &state::OutboxEntry {
                    id: String::new(),
                    provider: GOOGLE_PROVIDER.to_string(),
                    calendar_id: conflict.calendar_id.clone(),
                    event_id: existing_event.id.clone(),
                    operation,
                    enqueued_at: String::new(),
                    attempt_count: 0,
                    last_attempt_at: None,
                    last_error_code: None,
                    last_error_message: None,
                },
            )?;
        }
        ConflictResolutionStrategy::KeepRemote => {
            if is_cancelled(&remote_event) {
                db::soft_delete_event(&tx, &existing_event.id)?;
                state::delete_outbox_entry(&tx, GOOGLE_PROVIDER, &existing_event.id)?;
                state::upsert_event_sync_state(
                    &tx,
                    &EventSyncState {
                        event_id: existing_event.id.clone(),
                        sync_state: SyncState::Clean,
                        last_synced_at: Some(now_timestamp()),
                        last_push_attempt_at: None,
                        last_remote_modified_at: remote_updated_at(&remote_event),
                        last_sync_error_code: None,
                        last_sync_error_message: None,
                        remote_payload: Some(serde_json::to_string(&remote_event)?),
                        created_at: existing_state
                            .map(|state| state.created_at)
                            .unwrap_or_default(),
                        updated_at: String::new(),
                    },
                )?;
            } else {
                let merged = pull::merge_remote_event(
                    &conflict.calendar_id,
                    &remote_event,
                    Some(&existing_event),
                )?;
                db::update_event_from_sync(&tx, &merged)?;
                state::delete_outbox_entry(&tx, GOOGLE_PROVIDER, &existing_event.id)?;
                state::upsert_event_sync_state(
                    &tx,
                    &EventSyncState {
                        event_id: merged.id.clone(),
                        sync_state: SyncState::Clean,
                        last_synced_at: Some(now_timestamp()),
                        last_push_attempt_at: None,
                        last_remote_modified_at: remote_updated_at(&remote_event),
                        last_sync_error_code: None,
                        last_sync_error_message: None,
                        remote_payload: Some(serde_json::to_string(&remote_event)?),
                        created_at: existing_state
                            .as_ref()
                            .map(|state| state.created_at.clone())
                            .unwrap_or_default(),
                        updated_at: String::new(),
                    },
                )?;
            }
        }
    }

    state::resolve_conflict(&tx, conflict_id, strategy)?;
    let resolved = state::get_conflict(&tx, conflict_id)?
        .ok_or_else(|| anyhow!("conflict '{}' disappeared after resolve", conflict_id))?;
    tx.commit()?;
    Ok(resolved)
}

fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::resolve_conflict;
    use crate::{
        calendar_service::{self, CreateCalendarInput},
        db,
        google::types::{GoogleEvent, GoogleEventTime},
        models::{CalendarSource, Event},
        sync::state::{
            self, ConflictResolutionStatus, ConflictResolutionStrategy, EventSyncState,
            SyncConflict, SyncState,
        },
    };

    fn open_test_db() -> (TempDir, Connection) {
        let temp = TempDir::new().unwrap();
        let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
        (temp, conn)
    }

    fn create_google_calendar(conn: &Connection) -> crate::models::Calendar {
        calendar_service::create_calendar(
            conn,
            CreateCalendarInput {
                name: "Google".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("primary@example.com".to_string()),
                visible: true,
                position: None,
            },
        )
        .unwrap()
    }

    fn remote_event() -> GoogleEvent {
        GoogleEvent {
            id: Some("remote-1".to_string()),
            etag: Some("\"etag-2\"".to_string()),
            status: Some("confirmed".to_string()),
            summary: Some("Remote title".to_string()),
            description: None,
            location: None,
            start: Some(GoogleEventTime {
                date_time: Some("2026-04-12T09:00:00Z".to_string()),
                date: None,
                time_zone: Some("UTC".to_string()),
            }),
            end: Some(GoogleEventTime {
                date_time: Some("2026-04-12T10:00:00Z".to_string()),
                date: None,
                time_zone: Some("UTC".to_string()),
            }),
            recurrence: None,
            updated: Some("2026-04-12T08:00:00Z".to_string()),
            recurring_event_id: None,
            original_start_time: None,
        }
    }

    fn insert_conflict_state(
        conn: &Connection,
        calendar_id: &str,
        event: &Event,
        conflict_id: &str,
    ) {
        state::upsert_event_sync_state(
            conn,
            &EventSyncState {
                event_id: event.id.clone(),
                sync_state: SyncState::Conflicted,
                last_synced_at: None,
                last_push_attempt_at: None,
                last_remote_modified_at: None,
                last_sync_error_code: Some("412".to_string()),
                last_sync_error_message: Some("remote event changed before update".to_string()),
                remote_payload: None,
                created_at: String::new(),
                updated_at: String::new(),
            },
        )
        .unwrap();
        state::insert_conflict(
            conn,
            &SyncConflict {
                id: conflict_id.to_string(),
                event_id: event.id.clone(),
                calendar_id: calendar_id.to_string(),
                local_snapshot: serde_json::to_string(event).unwrap(),
                remote_snapshot: serde_json::to_string(&remote_event()).unwrap(),
                remote_etag: Some("\"etag-2\"".to_string()),
                detected_at: String::new(),
                resolution_status: ConflictResolutionStatus::Pending,
                resolution_strategy: None,
                resolved_at: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn keep_local_refreshes_google_version_and_requeues_update() {
        let (_temp, conn) = open_test_db();
        let calendar = create_google_calendar(&conn);
        let mut event = Event::new(
            calendar.id.clone(),
            "Local title",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.google_id = Some("remote-1".to_string());
        event.google_etag = Some("\"etag-1\"".to_string());
        db::insert_event(&conn, &event).unwrap();
        insert_conflict_state(&conn, &calendar.id, &event, "conflict-update");

        resolve_conflict(
            &conn,
            "conflict-update",
            ConflictResolutionStrategy::KeepLocal,
        )
        .unwrap();

        let refreshed = db::get_event_including_deleted(&conn, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(refreshed.google_etag.as_deref(), Some("\"etag-2\""));

        let outbox = state::get_outbox_entry(&conn, state::GOOGLE_PROVIDER, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(outbox.operation, state::OutboxOperation::Update);

        let sync_state = state::load_event_sync_state(&conn, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(sync_state.sync_state, SyncState::PendingUpdate);
    }

    #[test]
    fn keep_local_requeues_soft_deleted_events_as_deletes() {
        let (_temp, conn) = open_test_db();
        let calendar = create_google_calendar(&conn);
        let mut event = Event::new(
            calendar.id.clone(),
            "Local title",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.google_id = Some("remote-1".to_string());
        event.google_etag = Some("\"etag-1\"".to_string());
        db::insert_event(&conn, &event).unwrap();
        db::soft_delete_event(&conn, &event.id).unwrap();
        let deleted = db::get_event_including_deleted(&conn, &event.id)
            .unwrap()
            .unwrap();
        insert_conflict_state(&conn, &calendar.id, &deleted, "conflict-delete");

        resolve_conflict(
            &conn,
            "conflict-delete",
            ConflictResolutionStrategy::KeepLocal,
        )
        .unwrap();

        let refreshed = db::get_event_including_deleted(&conn, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(refreshed.google_etag.as_deref(), Some("\"etag-2\""));
        assert!(refreshed.deleted_at.is_some());

        let outbox = state::get_outbox_entry(&conn, state::GOOGLE_PROVIDER, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(outbox.operation, state::OutboxOperation::Delete);

        let sync_state = state::load_event_sync_state(&conn, &event.id)
            .unwrap()
            .unwrap();
        assert_eq!(sync_state.sync_state, SyncState::PendingDelete);
    }
}
