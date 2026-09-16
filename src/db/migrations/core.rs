use anyhow::Result;
use rusqlite::Connection;

use super::{base_schema::*, planner_schema::*};

pub(crate) const MIGRATION_V1: &str = "20260101000001";
pub(crate) const MIGRATION_V2: &str = "20260406000001";
pub(crate) const MIGRATION_V3: &str = "20260412000001";
pub(crate) const MIGRATION_V4: &str = "20260830000001";
pub(crate) const MIGRATION_V5: &str = "20260831000001";
pub(crate) const MIGRATION_V6: &str = "20260831000002";
pub(crate) const MIGRATION_V7: &str = "20260831000003";
pub(crate) const MIGRATION_V8: &str = "20260916000001";

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
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

    if !migration_applied(conn, MIGRATION_V4)? {
        migrate_v4(conn)?;
        record_migration(conn, MIGRATION_V4)?;
    }

    if !migration_applied(conn, MIGRATION_V5)? {
        migrate_v5(conn)?;
        record_migration(conn, MIGRATION_V5)?;
    }

    if !migration_applied(conn, MIGRATION_V6)? {
        migrate_v6(conn)?;
        record_migration(conn, MIGRATION_V6)?;
    }

    if !migration_applied(conn, MIGRATION_V7)? {
        migrate_v7(conn)?;
        record_migration(conn, MIGRATION_V7)?;
    }

    if !migration_applied(conn, MIGRATION_V8)? {
        migrate_v8(conn)?;
        record_migration(conn, MIGRATION_V8)?;
    }

    Ok(())
}

pub(crate) fn migration_applied(conn: &Connection, version: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            [version],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0)
}

pub(crate) fn record_migration(conn: &Connection, version: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (version) VALUES (?1)",
        [version],
    )?;
    Ok(())
}
