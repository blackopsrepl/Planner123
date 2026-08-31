use super::*;

pub fn create_task(
    conn: &Connection,
    input: CreateTaskInput,
) -> Result<PlanningTask, PlannerError> {
    validate_task_input(conn, &input)?;
    let now = now();
    let task = PlanningTask {
        id: Uuid::new_v4().to_string(),
        title: input.title.trim().to_string(),
        duration_minutes: input.duration_minutes,
        target_calendar_id: input.target_calendar_id,
        project_id: input.project_id,
        priority: input.priority,
        cognitive_load: input.cognitive_load,
        earliest_at: input.earliest_at,
        deadline_kind: input.deadline_kind,
        deadline_at: input.deadline_at,
        state: PlanningTaskState::Inbox,
        created_at: now.clone(),
        updated_at: now,
    };
    insert_task(conn, &task)?;
    Ok(task)
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<PlanningTask>, PlannerError> {
    let mut stmt = conn.prepare(
        "SELECT id,title,duration_minutes,target_calendar_id,project_id,priority,cognitive_load,
                earliest_at,deadline_kind,deadline_at,state,created_at,updated_at
         FROM planning_tasks ORDER BY state, created_at, title",
    ).map_err(internal)?;
    let rows = stmt.query_map([], task_from_row).map_err(internal)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)
}

pub fn get_task(conn: &Connection, id: &str) -> Result<Option<PlanningTask>, PlannerError> {
    conn.query_row(
        "SELECT id,title,duration_minutes,target_calendar_id,project_id,priority,cognitive_load,
                earliest_at,deadline_kind,deadline_at,state,created_at,updated_at
         FROM planning_tasks WHERE id=?1",
        [id],
        task_from_row,
    )
    .optional()
    .map_err(internal)
}

pub fn update_task(
    conn: &Connection,
    id: &str,
    update: UpdateTaskInput,
) -> Result<PlanningTask, PlannerError> {
    let mut task = require_task(conn, id)?;
    if task.state != PlanningTaskState::Inbox {
        return Err(PlannerError::Conflict(
            "applied tasks must be returned to the inbox before editing".into(),
        ));
    }
    if let Some(value) = update.title {
        task.title = value;
    }
    if let Some(value) = update.duration_minutes {
        task.duration_minutes = value;
    }
    if let Some(value) = update.target_calendar_id {
        task.target_calendar_id = value;
    }
    if let Some(value) = update.project_id {
        task.project_id = value;
    }
    if let Some(value) = update.priority {
        task.priority = value;
    }
    if let Some(value) = update.cognitive_load {
        task.cognitive_load = value;
    }
    if let Some(value) = update.earliest_at {
        task.earliest_at = value;
    }
    if let Some(value) = update.deadline_kind {
        task.deadline_kind = value;
    }
    if let Some(value) = update.deadline_at {
        task.deadline_at = value;
    }
    validate_task_input(
        conn,
        &CreateTaskInput {
            title: task.title.clone(),
            duration_minutes: task.duration_minutes,
            target_calendar_id: task.target_calendar_id.clone(),
            project_id: task.project_id.clone(),
            priority: task.priority.clone(),
            cognitive_load: task.cognitive_load.clone(),
            earliest_at: task.earliest_at.clone(),
            deadline_kind: task.deadline_kind.clone(),
            deadline_at: task.deadline_at.clone(),
        },
    )?;
    task.title = task.title.trim().to_string();
    task.updated_at = now();
    conn.execute("UPDATE planning_tasks SET title=?2,duration_minutes=?3,target_calendar_id=?4,project_id=?5,
        priority=?6,cognitive_load=?7,earliest_at=?8,deadline_kind=?9,deadline_at=?10,updated_at=?11 WHERE id=?1",
        params![task.id, task.title, task.duration_minutes, task.target_calendar_id, task.project_id,
            priority_db(&task.priority), cognitive_db(&task.cognitive_load), task.earliest_at,
            deadline_db(&task.deadline_kind), task.deadline_at, task.updated_at]).map_err(internal)?;
    require_task(conn, id)
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<(), PlannerError> {
    let task = require_task(conn, id)?;
    if task.state != PlanningTaskState::Inbox {
        return Err(PlannerError::Conflict(
            "return the task to the inbox before deleting it".into(),
        ));
    }
    conn.execute("DELETE FROM planning_tasks WHERE id=?1", [id])
        .map_err(internal)?;
    Ok(())
}

pub fn add_dependency(
    conn: &Connection,
    from_task_id: &str,
    to_task_id: &str,
) -> Result<(), PlannerError> {
    require_task(conn, from_task_id)?;
    require_task(conn, to_task_id)?;
    if from_task_id == to_task_id {
        return Err(PlannerError::Validation(
            "a task cannot block itself".into(),
        ));
    }
    conn.execute(
        "INSERT OR IGNORE INTO planning_task_dependencies (from_task_id,to_task_id) VALUES (?1,?2)",
        params![from_task_id, to_task_id],
    )
    .map_err(internal)?;
    if dependency_cycle(conn)? {
        conn.execute(
            "DELETE FROM planning_task_dependencies WHERE from_task_id=?1 AND to_task_id=?2",
            params![from_task_id, to_task_id],
        )
        .map_err(internal)?;
        return Err(PlannerError::Validation(
            "task dependency would create a cycle".into(),
        ));
    }
    Ok(())
}

pub fn remove_dependency(
    conn: &Connection,
    from_task_id: &str,
    to_task_id: &str,
) -> Result<(), PlannerError> {
    conn.execute(
        "DELETE FROM planning_task_dependencies WHERE from_task_id=?1 AND to_task_id=?2",
        params![from_task_id, to_task_id],
    )
    .map_err(internal)?;
    Ok(())
}

pub fn list_dependencies(conn: &Connection) -> Result<Vec<(String, String)>, PlannerError> {
    let mut stmt = conn.prepare("SELECT from_task_id,to_task_id FROM planning_task_dependencies ORDER BY from_task_id,to_task_id").map_err(internal)?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(internal)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)
}
