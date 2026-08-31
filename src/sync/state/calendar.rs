pub fn load_calendar_sync_state(
    conn: &Connection,
    calendar_id: &str,
) -> Result<Option<CalendarSyncState>> {
    conn.query_row(
        "SELECT css.calendar_id, css.google_access_role, css.writable, css.last_synced_at,
                css.last_sync_error_code, css.last_sync_error_message,
                (SELECT COUNT(*) FROM sync_outbox
                 WHERE provider = 'google' AND calendar_id = css.calendar_id),
                (SELECT COUNT(*) FROM sync_conflicts
                 WHERE calendar_id = css.calendar_id
                   AND resolution_status = 'pending'),
                css.created_at, css.updated_at
         FROM calendar_sync_state css
         WHERE css.calendar_id = ?1",
        [calendar_id],
        |row| {
            Ok(CalendarSyncState {
                calendar_id: row.get(0)?,
                google_access_role: row.get(1)?,
                writable: row.get::<_, i64>(2)? != 0,
                last_synced_at: row.get(3)?,
                last_sync_error_code: row.get(4)?,
                last_sync_error_message: row.get(5)?,
                pending_outbox: row.get::<_, i64>(6)? as usize,
                pending_conflicts: row.get::<_, i64>(7)? as usize,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn load_calendar_sync_states(conn: &Connection) -> Result<Vec<CalendarSyncState>> {
    let mut stmt = conn.prepare(
        "SELECT css.calendar_id, css.google_access_role, css.writable, css.last_synced_at,
                css.last_sync_error_code, css.last_sync_error_message,
                (SELECT COUNT(*) FROM sync_outbox
                 WHERE provider = 'google' AND calendar_id = css.calendar_id),
                (SELECT COUNT(*) FROM sync_conflicts
                 WHERE calendar_id = css.calendar_id
                   AND resolution_status = 'pending'),
                css.created_at, css.updated_at
         FROM calendar_sync_state css
         ORDER BY css.calendar_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CalendarSyncState {
            calendar_id: row.get(0)?,
            google_access_role: row.get(1)?,
            writable: row.get::<_, i64>(2)? != 0,
            last_synced_at: row.get(3)?,
            last_sync_error_code: row.get(4)?,
            last_sync_error_message: row.get(5)?,
            pending_outbox: row.get::<_, i64>(6)? as usize,
            pending_conflicts: row.get::<_, i64>(7)? as usize,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn upsert_calendar_sync_state(conn: &Connection, state: &CalendarSyncState) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO calendar_sync_state
             (calendar_id, google_access_role, writable, last_synced_at,
              last_sync_error_code, last_sync_error_message, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(calendar_id) DO UPDATE SET
             google_access_role = excluded.google_access_role,
             writable = excluded.writable,
             last_synced_at = excluded.last_synced_at,
             last_sync_error_code = excluded.last_sync_error_code,
             last_sync_error_message = excluded.last_sync_error_message,
             updated_at = excluded.updated_at",
        params![
            state.calendar_id,
            state.google_access_role,
            state.writable as i64,
            state.last_synced_at,
            state.last_sync_error_code,
            state.last_sync_error_message,
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
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::{now_timestamp, CalendarSyncState};
