pub fn load_events_in_range(conn: &Connection, from: &str, to: &str) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, calendar_id, project_id, title, description, location,
                start_at, end_at, all_day, rrule, google_id, google_etag,
                reminder_minutes, timezone, created_at, updated_at, deleted_at
         FROM events
         WHERE deleted_at IS NULL
           AND start_at <= ?2 AND end_at >= ?1
         ORDER BY start_at",
    )?;
    query_events(&mut stmt, &[from, to])
}

pub(crate) fn query_events(
    stmt: &mut rusqlite::Statement<'_>,
    params: &[&str],
) -> Result<Vec<Event>> {
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(Event {
            id: row.get(0)?,
            calendar_id: row.get(1)?,
            project_id: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            location: row.get(5)?,
            start_at: row.get(6)?,
            end_at: row.get(7)?,
            all_day: row.get::<_, i64>(8)? != 0,
            rrule: row.get(9)?,
            google_id: row.get(10)?,
            google_etag: row.get(11)?,
            reminder_minutes: row.get(12)?,
            timezone: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
            deleted_at: row.get(16)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn insert_event(conn: &Connection, ev: &Event) -> Result<()> {
    conn.execute(
        "INSERT INTO events
             (id, calendar_id, project_id, title, description, location,
              start_at, end_at, all_day, rrule, google_id, google_etag,
              reminder_minutes, timezone, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
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
            ev.created_at,
            ev.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_event(conn: &Connection, ev: &Event) -> Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE events SET
             calendar_id=?2, project_id=?3, title=?4, description=?5, location=?6,
             start_at=?7, end_at=?8, all_day=?9, rrule=?10, reminder_minutes=?11,
             timezone=?12, updated_at=?13
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
            ev.reminder_minutes,
            ev.timezone,
            now,
        ],
    )?;
    Ok(())
}

/* Soft-delete an event by setting deleted_at. */
pub fn soft_delete_event(conn: &Connection, event_id: &str) -> Result<()> {
    let now = now_timestamp();
    delete_dependencies_for_event(conn, event_id)?;
    conn.execute(
        "UPDATE events SET deleted_at=?2, updated_at=?2 WHERE id=?1",
        [event_id, &now],
    )?;
    Ok(())
}

pub fn load_events(conn: &Connection) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, calendar_id, project_id, title, description, location,
                start_at, end_at, all_day, rrule, google_id, google_etag,
                reminder_minutes, timezone, created_at, updated_at, deleted_at
         FROM events
         WHERE deleted_at IS NULL
         ORDER BY start_at, title",
    )?;
    query_events(&mut stmt, &[])
}
use anyhow::Result;
use rusqlite::Connection;

use crate::models::Event;

use crate::db::{calendar::now_timestamp, dependency::delete_dependencies_for_event};

/* Load events within a datetime window (inclusive). Excludes soft-deleted. */
