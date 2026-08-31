use super::*;

pub(super) fn solve(plan: SolverPlan) -> Result<SolverPlan, PlannerError> {
    let (job, mut events) = PLANNER_MANAGER
        .solve(plan)
        .map_err(|e| PlannerError::Internal(e.to_string()))?;
    while let Some(event) = events.blocking_recv() {
        match event {
            SolverEvent::Completed { solution, .. } => {
                PLANNER_MANAGER
                    .delete(job)
                    .map_err(|e| PlannerError::Internal(e.to_string()))?;
                return Ok(solution);
            }
            SolverEvent::Failed { error, .. } => {
                let _ = PLANNER_MANAGER.delete(job);
                return Err(PlannerError::Internal(format!(
                    "SolverForge failed: {error}"
                )));
            }
            SolverEvent::Cancelled { .. } => {
                let _ = PLANNER_MANAGER.delete(job);
                return Err(PlannerError::Conflict(
                    "SolverForge optimization was cancelled".into(),
                ));
            }
            _ => {}
        }
    }
    let _ = PLANNER_MANAGER.delete(job);
    Err(PlannerError::Internal(
        "SolverForge ended without a terminal result".into(),
    ))
}

pub(super) fn task_from_row(row: &Row<'_>) -> rusqlite::Result<PlanningTask> {
    Ok(PlanningTask {
        id: row.get(0)?,
        title: row.get(1)?,
        duration_minutes: row.get(2)?,
        target_calendar_id: row.get(3)?,
        project_id: row.get(4)?,
        priority: priority_from(&row.get::<_, String>(5)?),
        cognitive_load: cognitive_from(&row.get::<_, String>(6)?),
        earliest_at: row.get(7)?,
        deadline_kind: deadline_from(&row.get::<_, String>(8)?),
        deadline_at: row.get(9)?,
        state: task_state_from(&row.get::<_, String>(10)?),
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}
pub(super) fn proposal_from_row(row: &Row<'_>) -> rusqlite::Result<PlannerProposal> {
    Ok(PlannerProposal {
        id: row.get(0)?,
        status: row.get(1)?,
        horizon_start: row.get(2)?,
        horizon_days: row.get(3)?,
        timezone: row.get(4)?,
        score: row.get(5)?,
        created_at: row.get(6)?,
        applied_at: row.get(7)?,
    })
}
pub(super) fn proposal_item_from_row(row: &Row<'_>) -> rusqlite::Result<PlannerProposalItem> {
    let diagnostics_json: String = row.get(9)?;
    Ok(PlannerProposalItem {
        id: row.get(0)?,
        proposal_id: row.get(1)?,
        task_id: row.get(2)?,
        start_at: row.get(3)?,
        end_at: row.get(4)?,
        scheduled: row.get::<_, i64>(5)? != 0,
        cognitive_penalty: row.get(6)?,
        fatigue_penalty: row.get(7)?,
        explanation: row.get(8)?,
        diagnostics: serde_json::from_str(&diagnostics_json).unwrap_or_default(),
    })
}
pub(super) fn settings_from_row(row: &Row<'_>) -> rusqlite::Result<PlannerSettings> {
    Ok(PlannerSettings {
        timezone: row.get(0)?,
        availability_json: row.get(1)?,
        horizon_days: row.get(2)?,
        slot_minutes: row.get(3)?,
        solve_seconds: row.get(4)?,
        priority_low_weight: row.get(5)?,
        priority_normal_weight: row.get(6)?,
        priority_high_weight: row.get(7)?,
        cognitive_enabled: row.get::<_, i64>(8)? != 0,
        low_window_start: row.get(9)?,
        low_window_end: row.get(10)?,
        low_outside_penalty: row.get(11)?,
        medium_window_start: row.get(12)?,
        medium_window_end: row.get(13)?,
        medium_outside_penalty: row.get(14)?,
        high_window_start: row.get(15)?,
        high_window_end: row.get(16)?,
        high_outside_penalty: row.get(17)?,
        high_streak_limit: row.get(18)?,
        recovery_minutes: row.get(19)?,
        excess_high_penalty: row.get(20)?,
    })
}
pub(super) fn priority_db(value: &TaskPriority) -> &'static str {
    match value {
        TaskPriority::Low => "low",
        TaskPriority::Normal => "normal",
        TaskPriority::High => "high",
    }
}
pub(super) fn priority_from(value: &str) -> TaskPriority {
    match value {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        _ => TaskPriority::Normal,
    }
}
pub(super) fn cognitive_db(value: &CognitiveLoad) -> &'static str {
    match value {
        CognitiveLoad::Low => "low",
        CognitiveLoad::Medium => "medium",
        CognitiveLoad::High => "high",
    }
}
pub(super) fn cognitive_from(value: &str) -> CognitiveLoad {
    match value {
        "low" => CognitiveLoad::Low,
        "high" => CognitiveLoad::High,
        _ => CognitiveLoad::Medium,
    }
}
pub(super) fn deadline_db(value: &DeadlineKind) -> &'static str {
    match value {
        DeadlineKind::None => "none",
        DeadlineKind::Hard => "hard",
        DeadlineKind::Soft => "soft",
    }
}
pub(super) fn deadline_from(value: &str) -> DeadlineKind {
    match value {
        "hard" => DeadlineKind::Hard,
        "soft" => DeadlineKind::Soft,
        _ => DeadlineKind::None,
    }
}
pub(super) fn task_state_from(value: &str) -> PlanningTaskState {
    match value {
        "applied" => PlanningTaskState::Applied,
        "missing_event" => PlanningTaskState::MissingEvent,
        _ => PlanningTaskState::Inbox,
    }
}
pub(super) fn now() -> String {
    Utc::now().format(time::STORAGE_FORMAT).to_string()
}
pub(super) fn internal(error: impl std::fmt::Display) -> PlannerError {
    PlannerError::Internal(error.to_string())
}
pub(super) fn validation(error: anyhow::Error) -> PlannerError {
    PlannerError::Validation(error.to_string())
}
pub(super) fn event_service_error(error: event_service::EventServiceError) -> PlannerError {
    match error {
        event_service::EventServiceError::NotFound { resource, id } => {
            PlannerError::NotFound { resource, id }
        }
        event_service::EventServiceError::Validation(message) => PlannerError::Validation(message),
        event_service::EventServiceError::Conflict(message) => PlannerError::Conflict(message),
        event_service::EventServiceError::Internal(message) => PlannerError::Internal(message),
    }
}
