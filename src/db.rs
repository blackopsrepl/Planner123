use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, Row};

const MIGRATION_V1: &str = "20260101000001";
const MIGRATION_V2: &str = "20260406000001";
const MIGRATION_V3: &str = "20260412000001";

/* Path to the calendar database. */
pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solverforge")
        .join("calendar.db")
}

/* Open (or create) the database and run pending migrations. */
pub fn open() -> Result<Connection> {
    open_at(db_path())
}

pub fn open_at(path: impl AsRef<Path>) -> Result<Connection> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create data directory: {}", parent.display()))?;
    }

    let conn = Connection::open(path)
        .with_context(|| format!("cannot open database: {}", path.display()))?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )
    .context("cannot configure pragmas")?;

    migrate(&conn).context("schema migration failed")?;
    ensure_default_calendar_if_none_active(&conn).context("default calendar recovery failed")?;

    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    // Rails-compatible schema_migrations table.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY
        );",
    )?;

    if !migration_applied(conn, MIGRATION_V1)? {
        migrate_v1(conn)?;
        record_migration(conn, MIGRATION_V1)?;
    }

    if !migration_applied(conn, MIGRATION_V2)? {
        migrate_v2(conn)?;
        record_migration(conn, MIGRATION_V2)?;
    }

    if !migration_applied(conn, MIGRATION_V3)? {
        migrate_v3(conn)?;
        record_migration(conn, MIGRATION_V3)?;
    }

    Ok(())
}

fn migration_applied(conn: &Connection, version: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            [version],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0)
}

fn record_migration(conn: &Connection, version: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (version) VALUES (?1)",
        [version],
    )?;
    Ok(())
}

fn migrate_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- ── calendars ────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS calendars (
            id          TEXT PRIMARY KEY,          -- UUID v4
            name        TEXT    NOT NULL,
            color       TEXT    NOT NULL,          -- hex e.g. '#50f872'
            source      TEXT    NOT NULL DEFAULT 'local',  -- 'local' | 'google'
            google_id   TEXT,                      -- Google Calendar ID (for synced calendars)
            visible     INTEGER NOT NULL DEFAULT 1,
            position    INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            deleted_at  TEXT
        );

        -- ── projects ─────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS projects (
            id          TEXT PRIMARY KEY,
            name        TEXT    NOT NULL,
            color       TEXT    NOT NULL,
            description TEXT,
            created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            deleted_at  TEXT
        );

        -- ── events ───────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS events (
            id               TEXT PRIMARY KEY,
            calendar_id      TEXT    NOT NULL REFERENCES calendars(id),
            project_id       TEXT             REFERENCES projects(id),
            title            TEXT    NOT NULL,
            description      TEXT,
            location         TEXT,
            start_at         TEXT    NOT NULL,  -- ISO 8601 e.g. '2026-02-17 09:00:00'
            end_at           TEXT    NOT NULL,
            all_day          INTEGER NOT NULL DEFAULT 0,
            rrule            TEXT,              -- RFC 5545 RRULE string (recurring events)
            google_id        TEXT,              -- Google Event ID
            google_etag      TEXT,              -- Google ETag for conflict detection
            reminder_minutes INTEGER,           -- minutes before event; NULL = no reminder
            timezone         TEXT    NOT NULL DEFAULT 'UTC',
            created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            deleted_at       TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_events_calendar_id ON events(calendar_id);
        CREATE INDEX IF NOT EXISTS idx_events_project_id  ON events(project_id);
        CREATE INDEX IF NOT EXISTS idx_events_start_at    ON events(start_at);
        CREATE INDEX IF NOT EXISTS idx_events_end_at      ON events(end_at);
        CREATE INDEX IF NOT EXISTS idx_events_google_id   ON events(google_id)
            WHERE google_id IS NOT NULL;

        -- ── event_dependencies ───────────────────────────────────────
        -- DAG edges: from_event_id blocks to_event_id.
        -- Rails model: EventDependency
        CREATE TABLE IF NOT EXISTS event_dependencies (
            id              TEXT PRIMARY KEY,
            from_event_id   TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
            to_event_id     TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
            dependency_type TEXT NOT NULL DEFAULT 'blocks',  -- 'blocks' | 'related'
            created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            UNIQUE(from_event_id, to_event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_event_deps_from ON event_dependencies(from_event_id);
        CREATE INDEX IF NOT EXISTS idx_event_deps_to   ON event_dependencies(to_event_id);

        -- ── recurrence_exceptions ─────────────────────────────────────
        -- Modifies or deletes a single occurrence of a recurring event.
        CREATE TABLE IF NOT EXISTS recurrence_exceptions (
            id                   TEXT PRIMARY KEY,
            event_id             TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
            original_start       TEXT NOT NULL,     -- datetime of the overridden occurrence
            replacement_event_id TEXT REFERENCES events(id),  -- NULL = deleted occurrence
            created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
        );

        CREATE INDEX IF NOT EXISTS idx_recurrence_exc_event_id
            ON recurrence_exceptions(event_id);

        -- ── sync_tokens ──────────────────────────────────────────────
        -- Google Calendar incremental sync state per calendar.
        CREATE TABLE IF NOT EXISTS sync_tokens (
            id          TEXT PRIMARY KEY,
            calendar_id TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
            sync_token  TEXT NOT NULL,
            synced_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            UNIQUE(calendar_id)
        );
        ",
    )?;

    Ok(())
}

fn migrate_v2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS idx_calendars_google_id_unique
            ON calendars(google_id)
            WHERE deleted_at IS NULL
              AND source = 'google'
              AND google_id IS NOT NULL;

        CREATE UNIQUE INDEX IF NOT EXISTS idx_events_calendar_google_id_unique
            ON events(calendar_id, google_id)
            WHERE deleted_at IS NULL
              AND google_id IS NOT NULL;
        ",
    )?;

    Ok(())
}

fn migrate_v3(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS calendar_sync_state (
            calendar_id              TEXT PRIMARY KEY REFERENCES calendars(id) ON DELETE CASCADE,
            google_access_role       TEXT,
            writable                 INTEGER NOT NULL DEFAULT 1,
            last_synced_at           TEXT,
            last_sync_error_code     TEXT,
            last_sync_error_message  TEXT,
            created_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
        );

        CREATE TABLE IF NOT EXISTS event_sync_state (
            event_id                 TEXT PRIMARY KEY REFERENCES events(id) ON DELETE CASCADE,
            sync_state               TEXT NOT NULL DEFAULT 'clean',
            last_synced_at           TEXT,
            last_push_attempt_at     TEXT,
            last_remote_modified_at  TEXT,
            last_sync_error_code     TEXT,
            last_sync_error_message  TEXT,
            remote_payload           TEXT,
            created_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
        );

        CREATE TABLE IF NOT EXISTS sync_outbox (
            id                  TEXT PRIMARY KEY,
            provider            TEXT NOT NULL,
            calendar_id         TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
            event_id            TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
            operation           TEXT NOT NULL,
            enqueued_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            attempt_count       INTEGER NOT NULL DEFAULT 0,
            last_attempt_at     TEXT,
            last_error_code     TEXT,
            last_error_message  TEXT,
            UNIQUE(provider, event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_sync_outbox_provider_enqueued_at
            ON sync_outbox(provider, enqueued_at, id);

        CREATE TABLE IF NOT EXISTS sync_conflicts (
            id                   TEXT PRIMARY KEY,
            event_id             TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
            calendar_id          TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
            local_snapshot       TEXT NOT NULL,
            remote_snapshot      TEXT NOT NULL,
            remote_etag          TEXT,
            detected_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            resolution_status    TEXT NOT NULL DEFAULT 'pending',
            resolution_strategy  TEXT,
            resolved_at          TEXT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_conflicts_event_pending_unique
            ON sync_conflicts(event_id)
            WHERE resolution_status = 'pending';
        ",
    )?;

    Ok(())
}

// ── CRUD helpers ─────────────────────────────────────────────────────

use crate::models::{Calendar, CalendarSource, DependencyType, Event, EventDependency, Project};

fn calendar_from_row(row: &Row<'_>) -> rusqlite::Result<Calendar> {
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

fn now_timestamp() -> String {
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

pub fn load_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, description, created_at, updated_at, deleted_at
         FROM projects WHERE deleted_at IS NULL ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            description: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/* Load events within a datetime window (inclusive). Excludes soft-deleted. */
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

fn query_events(stmt: &mut rusqlite::Statement, params: &[&str]) -> Result<Vec<Event>> {
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

pub fn load_dependencies(conn: &Connection) -> Result<Vec<EventDependency>> {
    let mut stmt = conn.prepare(
        "SELECT id, from_event_id, to_event_id, dependency_type, created_at, updated_at
         FROM event_dependencies
         WHERE from_event_id IN (SELECT id FROM events WHERE deleted_at IS NULL)
           AND to_event_id IN (SELECT id FROM events WHERE deleted_at IS NULL)",
    )?;
    let rows = stmt.query_map([], |row| {
        let dep_type_str: String = row.get(3)?;
        let dependency_type = if dep_type_str == "blocks" {
            DependencyType::Blocks
        } else {
            DependencyType::Related
        };
        Ok(EventDependency {
            id: row.get(0)?,
            from_event_id: row.get(1)?,
            to_event_id: row.get(2)?,
            dependency_type,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
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

pub fn get_project(conn: &Connection, project_id: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, description, created_at, updated_at, deleted_at
         FROM projects
         WHERE id = ?1 AND deleted_at IS NULL",
    )?;

    let project = stmt
        .query_row([project_id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        })
        .optional()?;

    Ok(project)
}

pub fn insert_project(conn: &Connection, project: &Project) -> Result<()> {
    conn.execute(
        "INSERT INTO projects
             (id, name, color, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            project.id,
            project.name,
            project.color,
            project.description,
            project.created_at,
            project.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_project(conn: &Connection, project: &Project) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE projects SET
             name = ?2,
             color = ?3,
             description = ?4,
             updated_at = ?5
         WHERE id = ?1",
        rusqlite::params![
            project.id,
            project.name,
            project.color,
            project.description,
            now
        ],
    )?;
    Ok(())
}

pub fn soft_delete_project(conn: &Connection, project_id: &str) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE projects SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        [project_id, &now],
    )?;
    Ok(())
}

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

pub fn get_dependency(conn: &Connection, dependency_id: &str) -> Result<Option<EventDependency>> {
    let mut stmt = conn.prepare(
        "SELECT id, from_event_id, to_event_id, dependency_type, created_at, updated_at
         FROM event_dependencies
         WHERE id = ?1
           AND from_event_id IN (SELECT id FROM events WHERE deleted_at IS NULL)
           AND to_event_id IN (SELECT id FROM events WHERE deleted_at IS NULL)",
    )?;

    let dependency = stmt
        .query_row([dependency_id], |row| {
            let dep_type_str: String = row.get(3)?;
            let dependency_type = if dep_type_str == "blocks" {
                DependencyType::Blocks
            } else {
                DependencyType::Related
            };
            Ok(EventDependency {
                id: row.get(0)?,
                from_event_id: row.get(1)?,
                to_event_id: row.get(2)?,
                dependency_type,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .optional()?;

    Ok(dependency)
}

pub fn insert_dependency(conn: &Connection, dependency: &EventDependency) -> Result<()> {
    conn.execute(
        "INSERT INTO event_dependencies
             (id, from_event_id, to_event_id, dependency_type, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            dependency.id,
            dependency.from_event_id,
            dependency.to_event_id,
            dependency.dependency_type.to_string(),
            dependency.created_at,
            dependency.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_dependency(conn: &Connection, dependency: &EventDependency) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE event_dependencies SET
             from_event_id = ?2,
             to_event_id = ?3,
             dependency_type = ?4,
             updated_at = ?5
         WHERE id = ?1",
        rusqlite::params![
            dependency.id,
            dependency.from_event_id,
            dependency.to_event_id,
            dependency.dependency_type.to_string(),
            now,
        ],
    )?;
    Ok(())
}

pub fn delete_dependency(conn: &Connection, dependency_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM event_dependencies WHERE id = ?1",
        [dependency_id],
    )?;
    Ok(())
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

pub fn count_active_events_for_project(conn: &Connection, project_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM events WHERE project_id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

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

pub fn clear_project_id_for_project(conn: &Connection, project_id: &str) -> Result<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE events
         SET project_id = NULL, updated_at = ?2
         WHERE project_id = ?1 AND deleted_at IS NULL",
        [project_id, &now],
    )?;
    Ok(())
}

pub fn delete_dependencies_for_event(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM event_dependencies
         WHERE from_event_id = ?1 OR to_event_id = ?1",
        [event_id],
    )?;
    Ok(())
}

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

// Re-export rusqlite Optional trait for our use of .optional()
trait OptionalExt<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}
impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection as SqliteConnection;
    use tempfile::TempDir;

    #[test]
    fn open_at_reseeds_when_all_calendars_are_soft_deleted() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");

        {
            let conn = open_at(&db_path).unwrap();
            let calendars = load_calendars(&conn).unwrap();
            assert_eq!(calendars.len(), 1);
            soft_delete_calendar(&conn, &calendars[0].id).unwrap();
            assert_eq!(count_active_calendars(&conn).unwrap(), 0);
        }

        let reopened = open_at(&db_path).unwrap();
        let calendars = load_calendars(&reopened).unwrap();
        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].name, "Personal");
    }

    #[test]
    fn open_at_applies_google_sync_unique_indexes_to_existing_v1_db() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");

        {
            let conn = SqliteConnection::open(&db_path).unwrap();
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA synchronous = NORMAL;",
            )
            .unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations (
                    version TEXT PRIMARY KEY
                );",
            )
            .unwrap();
            migrate_v1(&conn).unwrap();
            record_migration(&conn, MIGRATION_V1).unwrap();
        }

        let reopened = open_at(&db_path).unwrap();
        let indexes = reopened
            .prepare("PRAGMA index_list('calendars')")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        let event_indexes = reopened
            .prepare("PRAGMA index_list('events')")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();

        assert!(indexes
            .iter()
            .any(|index| index == "idx_calendars_google_id_unique"));
        assert!(event_indexes
            .iter()
            .any(|index| index == "idx_events_calendar_google_id_unique"));
        assert!(migration_applied(&reopened, MIGRATION_V2).unwrap());
    }

    #[test]
    fn google_calendar_unique_index_blocks_duplicate_active_rows() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");
        let conn = open_at(&db_path).unwrap();
        let now = "2026-04-06 10:00:00".to_string();

        insert_calendar(
            &conn,
            &Calendar {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Work".to_string(),
                color: "#50f872".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: 1,
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
            },
        )
        .unwrap();

        let err = insert_calendar(
            &conn,
            &Calendar {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Work Copy".to_string(),
                color: "#ffaa00".to_string(),
                source: CalendarSource::Google,
                google_id: Some("work@example.com".to_string()),
                visible: true,
                position: 2,
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            },
        )
        .unwrap_err();

        let sqlite_err = err.downcast_ref::<rusqlite::Error>().unwrap();
        match sqlite_err {
            rusqlite::Error::SqliteFailure(code, _) => {
                assert_eq!(code.code, rusqlite::ErrorCode::ConstraintViolation);
            }
            other => panic!("expected sqlite constraint violation, got {other:?}"),
        }
    }

    #[test]
    fn event_google_id_unique_index_is_scoped_to_calendar() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");
        let conn = open_at(&db_path).unwrap();
        let now = "2026-04-06 10:00:00".to_string();
        let first_calendar = Calendar {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Work".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("work@example.com".to_string()),
            visible: true,
            position: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        };
        let second_calendar = Calendar {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Personal".to_string(),
            color: "#ffaa00".to_string(),
            source: CalendarSource::Google,
            google_id: Some("personal@example.com".to_string()),
            visible: true,
            position: 2,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        };
        insert_calendar(&conn, &first_calendar).unwrap();
        insert_calendar(&conn, &second_calendar).unwrap();

        let first_event = Event {
            id: uuid::Uuid::new_v4().to_string(),
            calendar_id: first_calendar.id.clone(),
            project_id: None,
            title: "Sync target".to_string(),
            description: None,
            location: None,
            start_at: "2026-04-06 10:00:00".to_string(),
            end_at: "2026-04-06 11:00:00".to_string(),
            all_day: false,
            rrule: None,
            google_id: Some("shared-google-event".to_string()),
            google_etag: Some("\"etag-1\"".to_string()),
            reminder_minutes: None,
            timezone: "UTC".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        };
        insert_event(&conn, &first_event).unwrap();

        let duplicate_same_calendar = insert_event(
            &conn,
            &Event {
                id: uuid::Uuid::new_v4().to_string(),
                calendar_id: first_calendar.id.clone(),
                project_id: None,
                title: "Duplicate".to_string(),
                description: None,
                location: None,
                start_at: "2026-04-06 12:00:00".to_string(),
                end_at: "2026-04-06 13:00:00".to_string(),
                all_day: false,
                rrule: None,
                google_id: Some("shared-google-event".to_string()),
                google_etag: Some("\"etag-2\"".to_string()),
                reminder_minutes: None,
                timezone: "UTC".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
            },
        )
        .unwrap_err();
        let sqlite_err = duplicate_same_calendar
            .downcast_ref::<rusqlite::Error>()
            .unwrap();
        match sqlite_err {
            rusqlite::Error::SqliteFailure(code, _) => {
                assert_eq!(code.code, rusqlite::ErrorCode::ConstraintViolation);
            }
            other => panic!("expected sqlite constraint violation, got {other:?}"),
        }

        insert_event(
            &conn,
            &Event {
                id: uuid::Uuid::new_v4().to_string(),
                calendar_id: second_calendar.id.clone(),
                project_id: None,
                title: "Allowed in other calendar".to_string(),
                description: None,
                location: None,
                start_at: "2026-04-06 12:00:00".to_string(),
                end_at: "2026-04-06 13:00:00".to_string(),
                all_day: false,
                rrule: None,
                google_id: Some("shared-google-event".to_string()),
                google_etag: Some("\"etag-3\"".to_string()),
                reminder_minutes: None,
                timezone: "UTC".to_string(),
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn detach_google_sync_state_for_calendar_clears_event_sync_fields_and_token() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("calendar.db");
        let conn = open_at(&db_path).unwrap();
        let now = "2026-04-06 10:00:00".to_string();
        let calendar = Calendar {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Work".to_string(),
            color: "#50f872".to_string(),
            source: CalendarSource::Google,
            google_id: Some("work@example.com".to_string()),
            visible: true,
            position: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
            deleted_at: None,
        };
        insert_calendar(&conn, &calendar).unwrap();
        insert_event(
            &conn,
            &Event {
                id: uuid::Uuid::new_v4().to_string(),
                calendar_id: calendar.id.clone(),
                project_id: None,
                title: "Imported".to_string(),
                description: None,
                location: None,
                start_at: "2026-04-06 10:00:00".to_string(),
                end_at: "2026-04-06 11:00:00".to_string(),
                all_day: false,
                rrule: None,
                google_id: Some("google-event-1".to_string()),
                google_etag: Some("\"etag-1\"".to_string()),
                reminder_minutes: None,
                timezone: "UTC".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
                deleted_at: None,
            },
        )
        .unwrap();
        upsert_sync_token(&conn, &calendar.id, "sync-token-1").unwrap();

        detach_google_sync_state_for_calendar(&conn, &calendar.id).unwrap();

        let detached_event = load_events(&conn)
            .unwrap()
            .into_iter()
            .find(|event| event.calendar_id == calendar.id)
            .unwrap();
        assert_eq!(detached_event.google_id, None);
        assert_eq!(detached_event.google_etag, None);
        assert_eq!(get_sync_token(&conn, &calendar.id).unwrap(), None);
    }
}
