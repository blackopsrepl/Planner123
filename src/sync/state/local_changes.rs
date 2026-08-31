pub fn clear_event_sync_tracking(conn: &Connection, event_id: &str) -> Result<()> {
    delete_outbox_entry(conn, GOOGLE_PROVIDER, event_id)?;
    delete_event_sync_state(conn, event_id)?;
    delete_conflicts_for_event(conn, event_id)?;
    Ok(())
}

pub fn google_calendar_writable(conn: &Connection, calendar: &Calendar) -> Result<bool> {
    if calendar.source != CalendarSource::Google {
        return Ok(true);
    }
    Ok(load_calendar_sync_state(conn, &calendar.id)?
        .map(|state| state.writable)
        .unwrap_or(true))
}

pub fn record_local_event_save(
    conn: &Connection,
    calendar: &Calendar,
    previous_event: Option<&Event>,
    event: &Event,
) -> Result<()> {
    if let Some(previous_event) = previous_event {
        if previous_event.calendar_id != event.calendar_id && previous_event.google_id.is_some() {
            bail!("moving synced Google events between calendars is not supported yet");
        }
    }

    if calendar.source != CalendarSource::Google {
        if event.google_id.is_none() {
            clear_event_sync_tracking(conn, &event.id)?;
        }
        return Ok(());
    }

    if !google_calendar_writable(conn, calendar)? {
        bail!("calendar '{}' is read-only", calendar.name);
    }

    let existing_outbox = get_outbox_entry(conn, GOOGLE_PROVIDER, &event.id)?;
    let operation = match existing_outbox.as_ref().map(|entry| &entry.operation) {
        Some(OutboxOperation::Create) => OutboxOperation::Create,
        _ if event.google_id.is_none() => OutboxOperation::Create,
        _ => OutboxOperation::Update,
    };
    let sync_state = match operation {
        OutboxOperation::Create => SyncState::PendingCreate,
        OutboxOperation::Update => SyncState::PendingUpdate,
        OutboxOperation::Delete => SyncState::PendingDelete,
    };

    let existing_state = load_event_sync_state(conn, &event.id)?;
    upsert_event_sync_state(
        conn,
        &EventSyncState {
            event_id: event.id.clone(),
            sync_state,
            last_synced_at: existing_state
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_push_attempt_at: existing_state
                .as_ref()
                .and_then(|state| state.last_push_attempt_at.clone()),
            last_remote_modified_at: existing_state
                .as_ref()
                .and_then(|state| state.last_remote_modified_at.clone()),
            last_sync_error_code: None,
            last_sync_error_message: None,
            remote_payload: existing_state.and_then(|state| state.remote_payload),
            created_at: String::new(),
            updated_at: String::new(),
        },
    )?;
    upsert_outbox_entry(
        conn,
        &OutboxEntry {
            id: String::new(),
            provider: GOOGLE_PROVIDER.to_string(),
            calendar_id: calendar.id.clone(),
            event_id: event.id.clone(),
            operation,
            enqueued_at: String::new(),
            attempt_count: 0,
            last_attempt_at: None,
            last_error_code: None,
            last_error_message: None,
        },
    )?;
    Ok(())
}

pub fn record_local_event_delete(
    conn: &Connection,
    calendar: &Calendar,
    event: &Event,
) -> Result<()> {
    if calendar.source != CalendarSource::Google {
        clear_event_sync_tracking(conn, &event.id)?;
        return Ok(());
    }

    if !google_calendar_writable(conn, calendar)? {
        bail!("calendar '{}' is read-only", calendar.name);
    }

    if event.google_id.is_none() {
        clear_event_sync_tracking(conn, &event.id)?;
        return Ok(());
    }

    let existing_state = load_event_sync_state(conn, &event.id)?;
    upsert_event_sync_state(
        conn,
        &EventSyncState {
            event_id: event.id.clone(),
            sync_state: SyncState::PendingDelete,
            last_synced_at: existing_state
                .as_ref()
                .and_then(|state| state.last_synced_at.clone()),
            last_push_attempt_at: existing_state
                .as_ref()
                .and_then(|state| state.last_push_attempt_at.clone()),
            last_remote_modified_at: existing_state
                .as_ref()
                .and_then(|state| state.last_remote_modified_at.clone()),
            last_sync_error_code: None,
            last_sync_error_message: None,
            remote_payload: existing_state.and_then(|state| state.remote_payload),
            created_at: String::new(),
            updated_at: String::new(),
        },
    )?;
    upsert_outbox_entry(
        conn,
        &OutboxEntry {
            id: String::new(),
            provider: GOOGLE_PROVIDER.to_string(),
            calendar_id: calendar.id.clone(),
            event_id: event.id.clone(),
            operation: OutboxOperation::Delete,
            enqueued_at: String::new(),
            attempt_count: 0,
            last_attempt_at: None,
            last_error_code: None,
            last_error_message: None,
        },
    )?;
    Ok(())
}

pub(crate) fn map_outbox_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OutboxEntry> {
    let operation: String = row.get(4)?;
    Ok(OutboxEntry {
        id: row.get(0)?,
        provider: row.get(1)?,
        calendar_id: row.get(2)?,
        event_id: row.get(3)?,
        operation: OutboxOperation::from_db(&operation),
        enqueued_at: row.get(5)?,
        attempt_count: row.get(6)?,
        last_attempt_at: row.get(7)?,
        last_error_code: row.get(8)?,
        last_error_message: row.get(9)?,
    })
}

pub(crate) fn map_conflict_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncConflict> {
    let resolution_status: String = row.get(7)?;
    let resolution_strategy: Option<String> = row.get(8)?;
    Ok(SyncConflict {
        id: row.get(0)?,
        event_id: row.get(1)?,
        calendar_id: row.get(2)?,
        local_snapshot: row.get(3)?,
        remote_snapshot: row.get(4)?,
        remote_etag: row.get(5)?,
        detected_at: row.get(6)?,
        resolution_status: ConflictResolutionStatus::from_db(&resolution_status),
        resolution_strategy: resolution_strategy.as_deref().map(|value| match value {
            "keep_remote" => ConflictResolutionStrategy::KeepRemote,
            _ => ConflictResolutionStrategy::KeepLocal,
        }),
        resolved_at: row.get(9)?,
    })
}

pub(crate) fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::models::{Calendar, CalendarSource, Event};

use super::{
    delete_conflicts_for_event, delete_event_sync_state, delete_outbox_entry, get_outbox_entry,
    load_calendar_sync_state, load_event_sync_state, upsert_event_sync_state, upsert_outbox_entry,
    ConflictResolutionStatus, ConflictResolutionStrategy, EventSyncState, OutboxEntry,
    OutboxOperation, SyncConflict, SyncState, GOOGLE_PROVIDER,
};
