use super::*;

pub(super) fn validate_task_input(
    conn: &Connection,
    input: &CreateTaskInput,
) -> Result<(), PlannerError> {
    if input.title.trim().is_empty() {
        return Err(PlannerError::Validation("title cannot be empty".into()));
    }
    if input.duration_minutes <= 0 {
        return Err(PlannerError::Validation(
            "duration_minutes must be positive".into(),
        ));
    }
    let calendar = db::get_calendar(conn, &input.target_calendar_id)
        .map_err(internal)?
        .ok_or_else(|| PlannerError::NotFound {
            resource: "calendar",
            id: input.target_calendar_id.clone(),
        })?;
    if calendar.source == CalendarSource::Google
        && !state::google_calendar_writable(conn, &calendar).map_err(internal)?
    {
        return Err(PlannerError::Conflict(
            "target Google calendar is read-only".into(),
        ));
    }
    if let Some(project) = input.project_id.as_deref() {
        if db::get_project(conn, project).map_err(internal)?.is_none() {
            return Err(PlannerError::NotFound {
                resource: "project",
                id: project.into(),
            });
        }
    }
    match (&input.deadline_kind, &input.deadline_at) {
        (DeadlineKind::None, Some(_)) => {
            return Err(PlannerError::Validation(
                "deadline_at requires hard or soft deadline_kind".into(),
            ))
        }
        (DeadlineKind::Hard | DeadlineKind::Soft, None) => {
            return Err(PlannerError::Validation(
                "hard and soft deadline kinds require deadline_at".into(),
            ))
        }
        _ => {}
    }
    for timestamp in [input.earliest_at.as_deref(), input.deadline_at.as_deref()]
        .into_iter()
        .flatten()
    {
        time::normalize_timestamp(timestamp).map_err(validation)?;
    }
    Ok(())
}
pub(super) fn insert_task(conn: &Connection, task: &PlanningTask) -> Result<(), PlannerError> {
    conn.execute("INSERT INTO planning_tasks (id,title,duration_minutes,target_calendar_id,project_id,priority,cognitive_load,earliest_at,deadline_kind,deadline_at,state,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![task.id,task.title,task.duration_minutes,task.target_calendar_id,task.project_id,priority_db(&task.priority),cognitive_db(&task.cognitive_load),task.earliest_at,deadline_db(&task.deadline_kind),task.deadline_at,match task.state{PlanningTaskState::Inbox=>"inbox",PlanningTaskState::Applied=>"applied",PlanningTaskState::MissingEvent=>"missing_event"},task.created_at,task.updated_at]).map_err(internal)?;
    Ok(())
}
pub(super) fn dependency_cycle(conn: &Connection) -> Result<bool, PlannerError> {
    let deps = list_dependencies(conn)?;
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (a, b) in deps {
        graph.entry(a).or_default().push(b);
    }
    fn visit(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visiting: &mut std::collections::HashSet<String>,
        done: &mut std::collections::HashSet<String>,
    ) -> bool {
        if done.contains(node) {
            return false;
        }
        if !visiting.insert(node.into()) {
            return true;
        }
        for next in graph.get(node).into_iter().flatten() {
            if visit(next, graph, visiting, done) {
                return true;
            }
        }
        visiting.remove(node);
        done.insert(node.into());
        false
    }
    let mut visiting = std::collections::HashSet::new();
    let mut done = std::collections::HashSet::new();
    Ok(graph
        .keys()
        .any(|node| visit(node, &graph, &mut visiting, &mut done)))
}
pub(super) fn require_google_checkpoint(conn: &Connection) -> Result<(), PlannerError> {
    for calendar in db::load_calendars(conn)
        .map_err(internal)?
        .into_iter()
        .filter(|calendar| calendar.source == CalendarSource::Google)
    {
        let state = state::load_calendar_sync_state(conn, &calendar.id).map_err(internal)?;
        if state.and_then(|value| value.last_synced_at).is_none() {
            return Err(PlannerError::Conflict(format!(
                "Google calendar '{}' needs an explicit successful sync before optimization",
                calendar.name
            )));
        }
    }
    Ok(())
}
