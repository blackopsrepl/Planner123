use super::*;

pub fn list_proposals(conn: &Connection) -> Result<Vec<PlannerProposal>, PlannerError> {
    let mut stmt = conn.prepare("SELECT id,status,horizon_start,horizon_days,timezone,score,created_at,applied_at FROM planner_proposals ORDER BY created_at DESC").map_err(internal)?;
    let rows = stmt.query_map([], proposal_from_row).map_err(internal)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)
}

pub fn proposal(conn: &Connection, id: &str) -> Result<ProposalDetail, PlannerError> {
    let proposal = conn.query_row("SELECT id,status,horizon_start,horizon_days,timezone,score,created_at,applied_at FROM planner_proposals WHERE id=?1", [id], proposal_from_row).optional().map_err(internal)?
        .ok_or_else(|| PlannerError::NotFound { resource: "proposal", id: id.into() })?;
    let mut stmt = conn.prepare("SELECT id,proposal_id,task_id,start_at,end_at,scheduled,cognitive_penalty,fatigue_penalty,explanation,diagnostics_json FROM planner_proposal_items WHERE proposal_id=?1 ORDER BY start_at,task_id").map_err(internal)?;
    let items = stmt
        .query_map([id], proposal_item_from_row)
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;
    Ok(ProposalDetail {
        applicability: proposal_applicability(conn, id)?,
        proposal,
        items,
    })
}

pub fn apply_proposal(conn: &Connection, id: &str) -> Result<ProposalDetail, PlannerError> {
    let tx = conn.unchecked_transaction().map_err(internal)?;
    let detail = proposal(&tx, id)?;
    if detail.proposal.status != "ready" {
        return Err(PlannerError::Conflict(
            "proposal is not ready to apply".into(),
        ));
    }
    validate_snapshot(&tx, id)?;
    let mut application = Vec::new();
    for item in detail.items.iter().filter(|item| item.scheduled) {
        let task = require_task(&tx, &item.task_id)?;
        if task.state != PlanningTaskState::Inbox {
            return Err(PlannerError::Conflict(format!(
                "task '{}' is no longer in the inbox",
                task.title
            )));
        }
        let start = item.start_at.as_deref().ok_or_else(|| {
            PlannerError::Internal("scheduled proposal item lacks start time".into())
        })?;
        let end = item.end_at.as_deref().ok_or_else(|| {
            PlannerError::Internal("scheduled proposal item lacks end time".into())
        })?;
        let mut event = Event::new(
            task.target_calendar_id.clone(),
            task.title.clone(),
            start,
            end,
            detail.proposal.timezone.clone(),
        );
        event.project_id = task.project_id.clone();
        application.push((item, task, event));
    }
    for (_item, task, event) in application {
        let saved = event_service::save_event_in_transaction(&tx, event, true)
            .map_err(event_service_error)?;
        tx.execute(
            "INSERT INTO planning_task_events (task_id,event_id,proposal_id) VALUES (?1,?2,?3)",
            params![task.id, saved.id, id],
        )
        .map_err(internal)?;
        tx.execute(
            "UPDATE planning_tasks SET state='applied',updated_at=?2 WHERE id=?1",
            params![task.id, now()],
        )
        .map_err(internal)?;
    }
    tx.execute(
        "UPDATE planner_proposals SET status='applied',applied_at=?2 WHERE id=?1",
        params![id, now()],
    )
    .map_err(internal)?;
    tx.commit().map_err(internal)?;
    proposal(conn, id)
}

pub fn return_to_inbox(conn: &Connection, task_id: &str) -> Result<PlanningTask, PlannerError> {
    let tx = conn.unchecked_transaction().map_err(internal)?;
    let task = require_task(&tx, task_id)?;
    let event_id: Option<String> = tx
        .query_row(
            "SELECT event_id FROM planning_task_events WHERE task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(internal)?;
    if let Some(event_id) = event_id {
        if db::get_event(&tx, &event_id).map_err(internal)?.is_some() {
            event_service::delete_event_in_transaction(&tx, &event_id)
                .map_err(event_service_error)?;
        }
        tx.execute(
            "DELETE FROM planning_task_events WHERE task_id=?1",
            [task_id],
        )
        .map_err(internal)?;
    }
    tx.execute(
        "UPDATE planning_tasks SET state='inbox',updated_at=?2 WHERE id=?1",
        params![task_id, now()],
    )
    .map_err(internal)?;
    tx.commit().map_err(internal)?;
    require_task(conn, &task.id)
}
