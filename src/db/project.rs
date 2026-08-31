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

pub fn count_active_events_for_project(conn: &Connection, project_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM events WHERE project_id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| row.get(0),
    )
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
use anyhow::Result;
use rusqlite::Connection;

use crate::models::Project;

use super::{calendar::now_timestamp, utils::OptionalExt};
