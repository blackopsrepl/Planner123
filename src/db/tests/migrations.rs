use super::super::migrations::{
    migrate_v1, migration_applied, record_migration, MIGRATION_V1, MIGRATION_V2, MIGRATION_V6,
};
use super::super::*;
use crate::models::Event;
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
