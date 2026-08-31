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

pub fn delete_dependencies_for_event(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM event_dependencies
         WHERE from_event_id = ?1 OR to_event_id = ?1",
        [event_id],
    )?;
    Ok(())
}
use anyhow::Result;
use rusqlite::Connection;

use crate::models::{DependencyType, EventDependency};

use super::{calendar::now_timestamp, utils::OptionalExt};
