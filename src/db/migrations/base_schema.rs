use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate_v1(conn: &Connection) -> Result<()> {
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

pub(super) fn migrate_v2(conn: &Connection) -> Result<()> {
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

pub(super) fn migrate_v3(conn: &Connection) -> Result<()> {
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
