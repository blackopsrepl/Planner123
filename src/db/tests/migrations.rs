use super::super::migrations::{
    migrate_v1, migration_applied, record_migration, MIGRATION_V1, MIGRATION_V2, MIGRATION_V6,
    MIGRATION_V8,
};
use super::super::*;
use crate::models::{Event, PlannerProposalOutcome};
use crate::planner;
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
fn open_at_migrates_prefixed_rrules_to_canonical_content() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");

    {
        let conn = open_at(&db_path).unwrap();
        let calendar_id = load_calendars(&conn).unwrap()[0].id.clone();
        let mut event = Event::new(
            calendar_id,
            "Planning",
            "2026-04-12 09:00:00",
            "2026-04-12 10:00:00",
            "UTC",
        );
        event.rrule = Some("RRULE:FREQ=WEEKLY".to_string());
        insert_event(&conn, &event).unwrap();
        conn.execute(
            "DELETE FROM schema_migrations WHERE version = ?1",
            [MIGRATION_V6],
        )
        .unwrap();
    }

    let reopened = open_at(&db_path).unwrap();
    assert_eq!(
        load_events(&reopened).unwrap()[0].rrule.as_deref(),
        Some("FREQ=WEEKLY")
    );
    assert!(migration_applied(&reopened, MIGRATION_V6).unwrap());
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

/// Builds a pre-v8 proposal whose items carry the exact v7 legacy `{}`.
fn legacy_proposal_fixture(conn: &SqliteConnection) {
    let calendar_id = load_calendars(conn).unwrap()[0].id.clone();
    conn.execute(
        "INSERT INTO planning_tasks (id,title,duration_minutes,target_calendar_id) VALUES
            ('task-1','Legacy scheduled',60,?1),
            ('task-2','Legacy unassigned',60,?1)",
        [calendar_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO planner_proposals (id,status,horizon_start,horizon_days,timezone,snapshot_json) VALUES
            ('proposal','ready','2026-09-01 00:00:00',1,'UTC','{\"task_versions\":{},\"event_versions\":{}}')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO planner_proposal_items (id,proposal_id,task_id,start_at,end_at,scheduled,diagnostics_json) VALUES
            ('item-1','proposal','task-1','2026-09-01 09:00:00','2026-09-01 10:00:00',1,'{}'),
            ('item-2','proposal','task-2',NULL,NULL,0,'{}')",
        [],
    )
    .unwrap();
}

#[test]
fn open_at_migrates_legacy_empty_diagnostics_to_typed_outcomes() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");

    {
        let conn = open_at(&db_path).unwrap();
        legacy_proposal_fixture(&conn);
        conn.execute(
            "DELETE FROM schema_migrations WHERE version = ?1",
            [MIGRATION_V8],
        )
        .unwrap();
    }

    let reopened = open_at(&db_path).unwrap();
    assert!(migration_applied(&reopened, MIGRATION_V8).unwrap());
    let detail = planner::proposal(&reopened, "proposal").unwrap();
    // Items sort by start_at ASC, and SQLite orders NULLs first.
    let outcomes: Vec<_> = detail
        .items
        .iter()
        .map(|item| item.diagnostics.outcome.clone())
        .collect();
    assert_eq!(
        outcomes,
        vec![
            PlannerProposalOutcome::Unassigned,
            PlannerProposalOutcome::Scheduled
        ]
    );
    assert!(detail.items[0].diagnostics.busy_blockers.is_empty());
    assert_eq!(detail.items[0].diagnostics.busy_blockers_omitted, 0);
}

#[test]
fn proposal_diagnostics_stay_strict_for_unknown_json_after_migration() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("calendar.db");

    {
        let conn = open_at(&db_path).unwrap();
        legacy_proposal_fixture(&conn);
        conn.execute(
            "UPDATE planner_proposal_items SET diagnostics_json='not-json' WHERE id='item-1'",
            [],
        )
        .unwrap();
        conn.execute(
            "DELETE FROM schema_migrations WHERE version = ?1",
            [MIGRATION_V8],
        )
        .unwrap();
    }

    let reopened = open_at(&db_path).unwrap();
    assert!(planner::proposal(&reopened, "proposal").is_err());
}
