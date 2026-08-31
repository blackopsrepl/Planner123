pub fn delete_sync_token(conn: &Connection, calendar_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM sync_tokens WHERE calendar_id = ?1",
        [calendar_id],
    )?;
    Ok(())
}

pub fn detach_google_sync_state_for_calendar(conn: &Connection, calendar_id: &str) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE events
         SET google_id = NULL,
             google_etag = NULL,
             updated_at = ?2
         WHERE calendar_id = ?1
           AND (google_id IS NOT NULL OR google_etag IS NOT NULL)",
        [calendar_id, &now],
    )?;
    conn.execute(
        "DELETE FROM sync_outbox
         WHERE calendar_id = ?1",
        [calendar_id],
    )?;
    conn.execute(
        "DELETE FROM event_sync_state
         WHERE event_id IN (SELECT id FROM events WHERE calendar_id = ?1)",
        [calendar_id],
    )?;
    conn.execute(
        "DELETE FROM sync_conflicts
         WHERE calendar_id = ?1",
        [calendar_id],
    )?;
    conn.execute(
        "DELETE FROM calendar_sync_state
         WHERE calendar_id = ?1",
        [calendar_id],
    )?;
    delete_sync_token(conn, calendar_id)?;
    Ok(())
}

pub fn upsert_sync_token(conn: &Connection, calendar_id: &str, token: &str) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO sync_tokens (id, calendar_id, sync_token, synced_at)
         VALUES (?1,?2,?3,?4)
         ON CONFLICT(calendar_id) DO UPDATE SET sync_token=?3, synced_at=?4",
        rusqlite::params![id, calendar_id, token, now],
    )?;
    Ok(())
}

pub fn get_sync_token(conn: &Connection, calendar_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT sync_token FROM sync_tokens WHERE calendar_id=?1",
        [calendar_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

use anyhow::Result;
use rusqlite::Connection;

use super::{calendar::now_timestamp, utils::OptionalExt};
