pub fn load_event_sync_state(conn: &Connection, event_id: &str) -> Result<Option<EventSyncState>> {
    conn.query_row(
        "SELECT event_id, sync_state, last_synced_at, last_push_attempt_at,
                last_remote_modified_at, last_sync_error_code, last_sync_error_message,
                remote_payload, created_at, updated_at
         FROM event_sync_state
         WHERE event_id = ?1",
        [event_id],
        |row| {
            let sync_state: String = row.get(1)?;
            Ok(EventSyncState {
                event_id: row.get(0)?,
                sync_state: SyncState::from_db(&sync_state),
                last_synced_at: row.get(2)?,
                last_push_attempt_at: row.get(3)?,
                last_remote_modified_at: row.get(4)?,
                last_sync_error_code: row.get(5)?,
                last_sync_error_message: row.get(6)?,
                remote_payload: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn upsert_event_sync_state(conn: &Connection, state: &EventSyncState) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO event_sync_state
             (event_id, sync_state, last_synced_at, last_push_attempt_at,
              last_remote_modified_at, last_sync_error_code, last_sync_error_message,
              remote_payload, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(event_id) DO UPDATE SET
             sync_state = excluded.sync_state,
             last_synced_at = excluded.last_synced_at,
             last_push_attempt_at = excluded.last_push_attempt_at,
             last_remote_modified_at = excluded.last_remote_modified_at,
             last_sync_error_code = excluded.last_sync_error_code,
             last_sync_error_message = excluded.last_sync_error_message,
             remote_payload = excluded.remote_payload,
             updated_at = excluded.updated_at",
        params![
            state.event_id,
            state.sync_state.to_string(),
            state.last_synced_at,
            state.last_push_attempt_at,
            state.last_remote_modified_at,
            state.last_sync_error_code,
            state.last_sync_error_message,
            state.remote_payload,
            if state.created_at.is_empty() {
                now.clone()
            } else {
                state.created_at.clone()
            },
            now,
        ],
    )?;
    Ok(())
}

pub fn delete_event_sync_state(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM event_sync_state WHERE event_id = ?1",
        [event_id],
    )?;
    Ok(())
}
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::{now_timestamp, EventSyncState, SyncState};
