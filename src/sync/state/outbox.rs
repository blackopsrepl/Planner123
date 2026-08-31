pub fn load_outbox_entries(conn: &Connection, provider: Option<&str>) -> Result<Vec<OutboxEntry>> {
    let sql = if provider.is_some() {
        "SELECT id, provider, calendar_id, event_id, operation, enqueued_at,
                attempt_count, last_attempt_at, last_error_code, last_error_message
         FROM sync_outbox
         WHERE provider = ?1
         ORDER BY enqueued_at, id"
    } else {
        "SELECT id, provider, calendar_id, event_id, operation, enqueued_at,
                attempt_count, last_attempt_at, last_error_code, last_error_message
         FROM sync_outbox
         ORDER BY enqueued_at, id"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(provider) = provider {
        stmt.query_map([provider], map_outbox_row)?
    } else {
        stmt.query_map([], map_outbox_row)?
    };
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn get_outbox_entry(
    conn: &Connection,
    provider: &str,
    event_id: &str,
) -> Result<Option<OutboxEntry>> {
    conn.query_row(
        "SELECT id, provider, calendar_id, event_id, operation, enqueued_at,
                attempt_count, last_attempt_at, last_error_code, last_error_message
         FROM sync_outbox
         WHERE provider = ?1 AND event_id = ?2",
        params![provider, event_id],
        map_outbox_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn upsert_outbox_entry(conn: &Connection, entry: &OutboxEntry) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO sync_outbox
             (id, provider, calendar_id, event_id, operation, enqueued_at,
              attempt_count, last_attempt_at, last_error_code, last_error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(provider, event_id) DO UPDATE SET
             calendar_id = excluded.calendar_id,
             operation = excluded.operation,
             enqueued_at = excluded.enqueued_at,
             last_error_code = NULL,
             last_error_message = NULL",
        params![
            if entry.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                entry.id.clone()
            },
            entry.provider,
            entry.calendar_id,
            entry.event_id,
            entry.operation.to_string(),
            if entry.enqueued_at.is_empty() {
                now
            } else {
                entry.enqueued_at.clone()
            },
            entry.attempt_count,
            entry.last_attempt_at,
            entry.last_error_code,
            entry.last_error_message,
        ],
    )?;
    Ok(())
}

pub fn delete_outbox_entry(conn: &Connection, provider: &str, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM sync_outbox WHERE provider = ?1 AND event_id = ?2",
        params![provider, event_id],
    )?;
    Ok(())
}
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::{map_outbox_row, now_timestamp, OutboxEntry};
