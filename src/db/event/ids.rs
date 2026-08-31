pub fn load_active_event_ids_for_calendar(
    conn: &Connection,
    calendar_id: &str,
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM events WHERE calendar_id = ?1 AND deleted_at IS NULL ORDER BY start_at, title",
    )?;
    let rows = stmt.query_map([calendar_id], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
use anyhow::Result;
use rusqlite::Connection;
