pub fn get_event(conn: &Connection, event_id: &str) -> Result<Option<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, calendar_id, project_id, title, description, location,
                start_at, end_at, all_day, rrule, google_id, google_etag,
                reminder_minutes, timezone, created_at, updated_at, deleted_at
         FROM events
         WHERE id = ?1 AND deleted_at IS NULL",
    )?;

    let rows = query_events(&mut stmt, &[event_id])?;
    Ok(rows.into_iter().next())
}

pub fn get_event_including_deleted(conn: &Connection, event_id: &str) -> Result<Option<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, calendar_id, project_id, title, description, location,
                start_at, end_at, all_day, rrule, google_id, google_etag,
                reminder_minutes, timezone, created_at, updated_at, deleted_at
         FROM events
         WHERE id = ?1",
    )?;

    let rows = query_events(&mut stmt, &[event_id])?;
    Ok(rows.into_iter().next())
}

pub fn get_event_by_google_id(
    conn: &Connection,
    calendar_id: &str,
    google_id: &str,
    include_deleted: bool,
) -> Result<Option<Event>> {
    let sql = if include_deleted {
        "SELECT id, calendar_id, project_id, title, description, location,
                start_at, end_at, all_day, rrule, google_id, google_etag,
                reminder_minutes, timezone, created_at, updated_at, deleted_at
         FROM events
         WHERE calendar_id = ?1
           AND google_id = ?2"
    } else {
        "SELECT id, calendar_id, project_id, title, description, location,
                start_at, end_at, all_day, rrule, google_id, google_etag,
                reminder_minutes, timezone, created_at, updated_at, deleted_at
         FROM events
         WHERE calendar_id = ?1
           AND google_id = ?2
           AND deleted_at IS NULL"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = query_events(&mut stmt, &[calendar_id, google_id])?;
    Ok(rows.into_iter().next())
}

pub fn update_event_from_sync(conn: &Connection, ev: &Event) -> Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE events SET
             calendar_id=?2, project_id=?3, title=?4, description=?5, location=?6,
             start_at=?7, end_at=?8, all_day=?9, rrule=?10, google_id=?11, google_etag=?12,
             reminder_minutes=?13, timezone=?14, updated_at=?15, deleted_at=?16
         WHERE id=?1",
        rusqlite::params![
            ev.id,
            ev.calendar_id,
            ev.project_id,
            ev.title,
            ev.description,
            ev.location,
            ev.start_at,
            ev.end_at,
            ev.all_day as i64,
            ev.rrule,
            ev.google_id,
            ev.google_etag,
            ev.reminder_minutes,
            ev.timezone,
            now,
            ev.deleted_at,
        ],
    )?;
    Ok(())
}

pub fn update_event_google_version(
    conn: &Connection,
    event_id: &str,
    google_id: Option<&str>,
    google_etag: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE events SET
             google_id = ?2,
             google_etag = ?3,
             updated_at = ?4
         WHERE id = ?1",
        rusqlite::params![event_id, google_id, google_etag, now],
    )?;
    Ok(())
}
use anyhow::Result;
use rusqlite::Connection;

use crate::models::Event;

use super::basic::query_events;
