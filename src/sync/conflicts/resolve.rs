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
