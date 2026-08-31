pub fn load_conflicts(conn: &Connection, pending_only: bool) -> Result<Vec<SyncConflict>> {
    let sql = if pending_only {
        "SELECT id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
                detected_at, resolution_status, resolution_strategy, resolved_at
         FROM sync_conflicts
         WHERE resolution_status = 'pending'
         ORDER BY detected_at DESC, id DESC"
    } else {
        "SELECT id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
                detected_at, resolution_status, resolution_strategy, resolved_at
         FROM sync_conflicts
         ORDER BY detected_at DESC, id DESC"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], map_conflict_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn get_conflict(conn: &Connection, conflict_id: &str) -> Result<Option<SyncConflict>> {
    conn.query_row(
        "SELECT id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
                detected_at, resolution_status, resolution_strategy, resolved_at
         FROM sync_conflicts
         WHERE id = ?1",
        [conflict_id],
        map_conflict_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn insert_conflict(conn: &Connection, conflict: &SyncConflict) -> Result<()> {
    conn.execute(
        "INSERT INTO sync_conflicts
             (id, event_id, calendar_id, local_snapshot, remote_snapshot, remote_etag,
              detected_at, resolution_status, resolution_strategy, resolved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            if conflict.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                conflict.id.clone()
            },
            conflict.event_id,
            conflict.calendar_id,
            conflict.local_snapshot,
            conflict.remote_snapshot,
            conflict.remote_etag,
            if conflict.detected_at.is_empty() {
                now_timestamp()
            } else {
                conflict.detected_at.clone()
            },
            conflict.resolution_status.to_string(),
            conflict
                .resolution_strategy
                .as_ref()
                .map(std::string::ToString::to_string),
            conflict.resolved_at,
        ],
    )?;
    Ok(())
}

pub fn resolve_conflict(
    conn: &Connection,
    conflict_id: &str,
    strategy: ConflictResolutionStrategy,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_conflicts
         SET resolution_status = 'resolved',
             resolution_strategy = ?2,
             resolved_at = ?3
         WHERE id = ?1",
        params![conflict_id, strategy.to_string(), now_timestamp()],
    )?;
    Ok(())
}

pub fn delete_conflicts_for_event(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute("DELETE FROM sync_conflicts WHERE event_id = ?1", [event_id])?;
    Ok(())
}

pub fn clear_calendar_sync_state(conn: &Connection, calendar_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM calendar_sync_state WHERE calendar_id = ?1",
        [calendar_id],
    )?;
    Ok(())
}
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::{map_conflict_row, now_timestamp, ConflictResolutionStrategy, SyncConflict};
