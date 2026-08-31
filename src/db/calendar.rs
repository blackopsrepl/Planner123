use anyhow::Result;
use rusqlite::{Connection, Row};

use crate::models::{Calendar, CalendarSource};

use super::utils::OptionalExt;

pub(crate) fn calendar_from_row(row: &Row<'_>) -> rusqlite::Result<Calendar> {
    let source_str: String = row.get(3)?;
    let source = if source_str == "google" {
        CalendarSource::Google
    } else {
        CalendarSource::Local
    };
    Ok(Calendar {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        source,
        google_id: row.get(4)?,
        visible: row.get::<_, i64>(5)? != 0,
        position: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        deleted_at: row.get(9)?,
    })
}

pub fn load_calendars(conn: &Connection) -> Result<Vec<Calendar>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, source, google_id, visible, position,
                created_at, updated_at, deleted_at
         FROM calendars
         WHERE deleted_at IS NULL
         ORDER BY position, name",
    )?;
    let rows = stmt.query_map([], calendar_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(crate) fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn insert_default_calendar(conn: &Connection) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO calendars (id, name, color, source, position)
         VALUES (?1, 'Personal', '#82FB9C', 'local', 0)",
        [&id],
    )?;
    Ok(())
}

pub fn ensure_default_calendar_if_none_active(conn: &Connection) -> Result<()> {
    if count_active_calendars(conn)? == 0 {
        insert_default_calendar(conn)?;
    }
    Ok(())
}

pub fn get_calendar(conn: &Connection, calendar_id: &str) -> Result<Option<Calendar>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, source, google_id, visible, position,
                created_at, updated_at, deleted_at
         FROM calendars
         WHERE id = ?1 AND deleted_at IS NULL",
    )?;

    let calendar = stmt
        .query_row([calendar_id], calendar_from_row)
        .optional()?;

    Ok(calendar)
}

pub fn get_google_calendar_by_google_id(
    conn: &Connection,
    google_id: &str,
) -> Result<Option<Calendar>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, source, google_id, visible, position,
                created_at, updated_at, deleted_at
         FROM calendars
         WHERE google_id = ?1
           AND source = 'google'
           AND deleted_at IS NULL",
    )?;

    stmt.query_row([google_id], calendar_from_row)
        .optional()
        .map_err(Into::into)
}

pub fn insert_calendar(conn: &Connection, calendar: &Calendar) -> Result<()> {
    conn.execute(
        "INSERT INTO calendars
             (id, name, color, source, google_id, visible, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            calendar.id,
            calendar.name,
            calendar.color,
            calendar.source.to_string(),
            calendar.google_id,
            calendar.visible as i64,
            calendar.position,
            calendar.created_at,
            calendar.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_calendar(conn: &Connection, calendar: &Calendar) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE calendars SET
             name = ?2,
             color = ?3,
             source = ?4,
             google_id = ?5,
             visible = ?6,
             position = ?7,
             updated_at = ?8
         WHERE id = ?1",
        rusqlite::params![
            calendar.id,
            calendar.name,
            calendar.color,
            calendar.source.to_string(),
            calendar.google_id,
            calendar.visible as i64,
            calendar.position,
            now,
        ],
    )?;
    Ok(())
}

pub fn soft_delete_calendar(conn: &Connection, calendar_id: &str) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE calendars SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        [calendar_id, &now],
    )?;
    Ok(())
}

pub fn next_calendar_position(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM calendars WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn count_active_events_for_calendar(conn: &Connection, calendar_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM events WHERE calendar_id = ?1 AND deleted_at IS NULL",
        [calendar_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn count_active_calendars(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM calendars WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .map_err(Into::into)
}
