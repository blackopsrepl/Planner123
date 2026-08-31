use anyhow::Result;
use rusqlite::Connection;

pub(super) fn migrate_v4(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS planner_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            timezone TEXT,
            availability_json TEXT NOT NULL DEFAULT '{}',
            horizon_days INTEGER NOT NULL DEFAULT 14,
            slot_minutes INTEGER NOT NULL DEFAULT 15,
            solve_seconds INTEGER NOT NULL DEFAULT 5,
            priority_low_weight INTEGER NOT NULL DEFAULT 1,
            priority_normal_weight INTEGER NOT NULL DEFAULT 5,
            priority_high_weight INTEGER NOT NULL DEFAULT 25,
            cognitive_enabled INTEGER NOT NULL DEFAULT 0,
            low_window_start TEXT NOT NULL DEFAULT '00:00',
            low_window_end TEXT NOT NULL DEFAULT '00:00',
            low_outside_penalty INTEGER NOT NULL DEFAULT 0,
            medium_window_start TEXT NOT NULL DEFAULT '00:00',
            medium_window_end TEXT NOT NULL DEFAULT '00:00',
            medium_outside_penalty INTEGER NOT NULL DEFAULT 0,
            high_window_start TEXT NOT NULL DEFAULT '00:00',
            high_window_end TEXT NOT NULL DEFAULT '00:00',
            high_outside_penalty INTEGER NOT NULL DEFAULT 0,
            high_streak_limit INTEGER NOT NULL DEFAULT 1,
            recovery_minutes INTEGER NOT NULL DEFAULT 30,
            excess_high_penalty INTEGER NOT NULL DEFAULT 60,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
        );
        INSERT OR IGNORE INTO planner_settings (id) VALUES (1);

        CREATE TABLE IF NOT EXISTS planning_tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
            target_calendar_id TEXT NOT NULL REFERENCES calendars(id),
            project_id TEXT REFERENCES projects(id),
            priority TEXT NOT NULL DEFAULT 'normal',
            cognitive_load TEXT NOT NULL DEFAULT 'medium',
            earliest_at TEXT,
            deadline_kind TEXT NOT NULL DEFAULT 'none',
            deadline_at TEXT,
            state TEXT NOT NULL DEFAULT 'inbox',
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_planning_tasks_state ON planning_tasks(state, updated_at);

        CREATE TABLE IF NOT EXISTS planning_task_dependencies (
            from_task_id TEXT NOT NULL REFERENCES planning_tasks(id) ON DELETE CASCADE,
            to_task_id TEXT NOT NULL REFERENCES planning_tasks(id) ON DELETE CASCADE,
            PRIMARY KEY (from_task_id, to_task_id),
            CHECK (from_task_id <> to_task_id)
        );

        CREATE TABLE IF NOT EXISTS planner_proposals (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            horizon_start TEXT NOT NULL,
            horizon_days INTEGER NOT NULL,
            timezone TEXT NOT NULL,
            score TEXT,
            snapshot_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
            applied_at TEXT
        );

        CREATE TABLE IF NOT EXISTS planner_proposal_items (
            id TEXT PRIMARY KEY,
            proposal_id TEXT NOT NULL REFERENCES planner_proposals(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL REFERENCES planning_tasks(id),
            start_at TEXT,
            end_at TEXT,
            scheduled INTEGER NOT NULL,
            cognitive_penalty INTEGER NOT NULL DEFAULT 0,
            fatigue_penalty INTEGER NOT NULL DEFAULT 0,
            explanation TEXT,
            UNIQUE(proposal_id, task_id)
        );

        CREATE TABLE IF NOT EXISTS planning_task_events (
            task_id TEXT PRIMARY KEY REFERENCES planning_tasks(id) ON DELETE CASCADE,
            event_id TEXT NOT NULL UNIQUE REFERENCES events(id),
            proposal_id TEXT NOT NULL REFERENCES planner_proposals(id),
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
        );
        ",
    )?;
    Ok(())
}

pub(super) fn migrate_v5(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TRIGGER IF NOT EXISTS planner_task_event_soft_deleted
        AFTER UPDATE OF deleted_at ON events
        WHEN OLD.deleted_at IS NULL AND NEW.deleted_at IS NOT NULL
        BEGIN
            UPDATE planning_tasks
            SET state = 'missing_event', updated_at = NEW.updated_at
            WHERE id IN (
                SELECT task_id FROM planning_task_events WHERE event_id = NEW.id
            )
              AND state = 'applied';
        END;
        ",
    )?;
    Ok(())
}

pub(super) fn migrate_v6(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE events SET rrule = substr(rrule, 7) WHERE rrule LIKE 'RRULE:%'",
        [],
    )?;
    Ok(())
}

pub(super) fn migrate_v7(conn: &Connection) -> Result<()> {
    // Keep human explanations intact while storing machine-readable proposal evidence.
    conn.execute(
        "ALTER TABLE planner_proposal_items ADD COLUMN diagnostics_json TEXT NOT NULL DEFAULT '{}'",
        [],
    )?;
    Ok(())
}
