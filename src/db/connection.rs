use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::{calendar::ensure_default_calendar_if_none_active, migrations};

/* Path to the calendar database. */
pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("planner123")
        .join("calendar.db")
}

/* Pre-rebrand database location; copied once into the new location. */
fn legacy_db_path() -> Option<PathBuf> {
    let path = dirs::data_dir()?.join("solverforge").join("calendar.db");
    path.is_file().then_some(path)
}

/* Copy the pre-rebrand database into the new location exactly once.

Uses `VACUUM INTO` so the copy is a consistent snapshot even when the legacy
database is in WAL mode. Fails loudly rather than silently starting fresh. */
fn migrate_legacy_database() -> Result<()> {
    let target = db_path();
    if target.exists() {
        return Ok(());
    }
    let Some(source) = legacy_db_path() else {
        return Ok(());
    };

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create data directory: {}", parent.display()))?;
    }

    let source_display = source.display();
    let target_display = target.display();
    let legacy = Connection::open_with_flags(&source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("cannot open legacy database: {source_display}"))?;
    legacy
        .execute("VACUUM INTO ?1", [target_display.to_string()])
        .with_context(|| {
            format!("cannot migrate legacy database {source_display} to {target_display}")
        })?;
    Ok(())
}

/* Open (or create) the database and run pending migrations. */
pub fn open() -> Result<Connection> {
    migrate_legacy_database()?;
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

    migrations::migrate(&conn).context("schema migration failed")?;
    ensure_default_calendar_if_none_active(&conn).context("default calendar recovery failed")?;

    Ok(conn)
}
