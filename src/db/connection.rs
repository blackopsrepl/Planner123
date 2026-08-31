use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::{calendar::ensure_default_calendar_if_none_active, migrations};

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

    migrations::migrate(&conn).context("schema migration failed")?;
    ensure_default_calendar_if_none_active(&conn).context("default calendar recovery failed")?;

    Ok(conn)
}
