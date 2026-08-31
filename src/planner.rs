use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use solverforge::SolverEvent;
use uuid::Uuid;

use crate::{
    db, event_service,
    models::{
        CalendarSource, CognitiveLoad, DeadlineKind, Event, PlannerProposal, PlannerProposalItem,
        PlannerSettings, PlanningTask, PlanningTaskState, TaskPriority,
    },
    planner_domain::{SolverPlan, SolverSlot, SolverTask, PLANNER_MANAGER},
    sync::state,
    time,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerError {
    NotFound { resource: &'static str, id: String },
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { resource, id } => write!(f, "{} '{}' not found", resource, id),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for PlannerError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Availability(pub BTreeMap<String, Vec<TimeWindow>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    pub title: String,
    pub duration_minutes: i64,
    pub target_calendar_id: String,
    pub project_id: Option<String>,
    pub priority: TaskPriority,
    pub cognitive_load: CognitiveLoad,
    pub earliest_at: Option<String>,
    pub deadline_kind: DeadlineKind,
    pub deadline_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub duration_minutes: Option<i64>,
    pub target_calendar_id: Option<String>,
    pub project_id: Option<Option<String>>,
    pub priority: Option<TaskPriority>,
    pub cognitive_load: Option<CognitiveLoad>,
    pub earliest_at: Option<Option<String>>,
    pub deadline_kind: Option<DeadlineKind>,
    pub deadline_at: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsUpdate {
    pub timezone: Option<String>,
    pub availability: Option<Availability>,
    pub horizon_days: Option<i64>,
    pub slot_minutes: Option<i64>,
    pub solve_seconds: Option<i64>,
    pub priority_low_weight: Option<i64>,
    pub priority_normal_weight: Option<i64>,
    pub priority_high_weight: Option<i64>,
    pub cognitive_enabled: Option<bool>,
    pub low_window_start: Option<String>,
    pub low_window_end: Option<String>,
    pub low_outside_penalty: Option<i64>,
    pub medium_window_start: Option<String>,
    pub medium_window_end: Option<String>,
    pub medium_outside_penalty: Option<i64>,
    pub high_window_start: Option<String>,
    pub high_window_end: Option<String>,
    pub high_outside_penalty: Option<i64>,
    pub high_streak_limit: Option<i64>,
    pub recovery_minutes: Option<i64>,
    pub excess_high_penalty: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalDetail {
    pub proposal: PlannerProposal,
    pub items: Vec<PlannerProposalItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProposalSnapshot {
    task_versions: HashMap<String, String>,
    event_versions: HashMap<String, String>,
}

pub fn settings(conn: &Connection) -> Result<PlannerSettings, PlannerError> {
    conn.query_row(
        "SELECT timezone, availability_json, horizon_days, slot_minutes, solve_seconds,
                priority_low_weight, priority_normal_weight, priority_high_weight,
                cognitive_enabled, low_window_start, low_window_end, low_outside_penalty,
                medium_window_start, medium_window_end, medium_outside_penalty,
                high_window_start, high_window_end, high_outside_penalty,
                high_streak_limit, recovery_minutes, excess_high_penalty
         FROM planner_settings WHERE id = 1",
        [],
        settings_from_row,
    )
    .map_err(internal)
}

pub fn update_settings(
    conn: &Connection,
    update: SettingsUpdate,
) -> Result<PlannerSettings, PlannerError> {
    let mut current = settings(conn)?;
    if let Some(value) = update.timezone {
        current.timezone = Some(normalize_timezone(&value)?);
    }
    if let Some(value) = update.availability {
        validate_availability(&value)?;
        current.availability_json = serde_json::to_string(&value).map_err(internal)?;
    }
    macro_rules! replace {
        ($field:ident) => {
            if let Some(value) = update.$field {
                current.$field = value;
            }
        };
    }
    replace!(horizon_days);
    replace!(slot_minutes);
    replace!(solve_seconds);
    replace!(priority_low_weight);
    replace!(priority_normal_weight);
    replace!(priority_high_weight);
    replace!(cognitive_enabled);
    replace!(low_outside_penalty);
    replace!(medium_outside_penalty);
    replace!(high_outside_penalty);
    replace!(high_streak_limit);
    replace!(recovery_minutes);
    replace!(excess_high_penalty);
    if let Some(value) = update.low_window_start {
        current.low_window_start = normalize_clock(&value)?;
    }
    if let Some(value) = update.low_window_end {
        current.low_window_end = normalize_clock(&value)?;
    }
    if let Some(value) = update.medium_window_start {
        current.medium_window_start = normalize_clock(&value)?;
    }
    if let Some(value) = update.medium_window_end {
        current.medium_window_end = normalize_clock(&value)?;
    }
    if let Some(value) = update.high_window_start {
        current.high_window_start = normalize_clock(&value)?;
    }
    if let Some(value) = update.high_window_end {
        current.high_window_end = normalize_clock(&value)?;
    }
    validate_settings(&current)?;
    conn.execute(
        "UPDATE planner_settings SET timezone=?1, availability_json=?2, horizon_days=?3, slot_minutes=?4,
             solve_seconds=?5, priority_low_weight=?6, priority_normal_weight=?7, priority_high_weight=?8,
             cognitive_enabled=?9, low_window_start=?10, low_window_end=?11, low_outside_penalty=?12,
             medium_window_start=?13, medium_window_end=?14, medium_outside_penalty=?15,
             high_window_start=?16, high_window_end=?17, high_outside_penalty=?18,
             high_streak_limit=?19, recovery_minutes=?20, excess_high_penalty=?21,
             updated_at=strftime('%Y-%m-%d %H:%M:%S', 'now') WHERE id=1",
        params![current.timezone, current.availability_json, current.horizon_days, current.slot_minutes,
            current.solve_seconds, current.priority_low_weight, current.priority_normal_weight,
            current.priority_high_weight, current.cognitive_enabled as i64, current.low_window_start,
            current.low_window_end, current.low_outside_penalty, current.medium_window_start,
            current.medium_window_end, current.medium_outside_penalty, current.high_window_start,
            current.high_window_end, current.high_outside_penalty, current.high_streak_limit,
            current.recovery_minutes, current.excess_high_penalty],
    ).map_err(internal)?;
    settings(conn)
}

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

pub fn optimize(
    conn: &Connection,
    horizon_days: Option<i64>,
) -> Result<ProposalDetail, PlannerError> {
    let mut settings = settings(conn)?;
    if let Some(days) = horizon_days {
        settings.horizon_days = days;
    }
    validate_settings(&settings)?;
    let timezone_name = settings.timezone.clone().ok_or_else(|| {
        PlannerError::Validation("configure planner timezone before optimizing".into())
    })?;
    let timezone = Tz::from_str(&timezone_name)
        .map_err(|_| PlannerError::Validation("invalid planner timezone".into()))?;
    let availability: Availability = serde_json::from_str(&settings.availability_json)
        .map_err(|e| PlannerError::Validation(format!("invalid planner availability: {e}")))?;
    validate_availability(&availability)?;
    require_google_checkpoint(conn)?;
    let tasks: Vec<_> = list_tasks(conn)?
        .into_iter()
        .filter(|task| task.state == PlanningTaskState::Inbox)
        .collect();
    if tasks.is_empty() {
        return Err(PlannerError::Validation(
            "the planner inbox is empty".into(),
        ));
    }
    let local_now = Utc::now().with_timezone(&timezone);
    let local_start = timezone
        .from_local_datetime(&local_now.date_naive().and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or_else(|| {
            PlannerError::Validation("planner horizon starts at an invalid local time".into())
        })?;
    let slots = make_slots(local_start, settings.horizon_days, settings.slot_minutes)?;
    let busy = busy_intervals(conn)?;
    let dependency_map = dependency_map(conn, &tasks)?;
    let applied_predecessor_ends = applied_predecessor_ends(conn, &tasks, &timezone_name)?;
    let applied_high = applied_high_intervals(conn)?;
    let mut assignments = Vec::new();
    for (id, task) in tasks.iter().enumerate() {
        let (window_start, window_end, outside_penalty) =
            cognitive_profile(&settings, &task.cognitive_load);
        let earliest = task
            .earliest_at
            .as_deref()
            .map(|value| time::resolve_utc_datetime(value, &timezone_name))
            .transpose()
            .map_err(validation)?;
        let deadline = task
            .deadline_at
            .as_deref()
            .map(|value| time::resolve_utc_datetime(value, &timezone_name))
            .transpose()
            .map_err(validation)?;
        let mut feasible = Vec::with_capacity(slots.len());
        let mut cognitive = Vec::with_capacity(slots.len());
        let mut external_fatigue = Vec::with_capacity(slots.len());
        for slot in &slots {
            let end = slot.start + Duration::minutes(task.duration_minutes);
            let allowed = available_interval(slot.start, end, &availability, timezone)
                && !busy
                    .iter()
                    .any(|(start, finish)| overlaps(slot.start, end, *start, *finish))
                && earliest.map(|value| slot.start >= value).unwrap_or(true)
                && applied_predecessor_ends
                    .get(&task.id)
                    .map(|value| slot.start >= *value)
                    .unwrap_or(true)
                && (task.deadline_kind != DeadlineKind::Hard
                    || deadline.map(|value| end <= value).unwrap_or(false));
            feasible.push(allowed);
            cognitive.push(if settings.cognitive_enabled {
                cognitive_cost(
                    slot.start,
                    end,
                    &window_start,
                    &window_end,
                    outside_penalty,
                    timezone,
                )
            } else {
                0
            });
            external_fatigue.push(applied_fatigue_cost(
                task.cognitive_load == CognitiveLoad::High,
                slot.start,
                &applied_high,
                &settings,
            ));
        }
        assignments.push(SolverTask {
            id,
            task_id: task.id.clone(),
            duration_minutes: task.duration_minutes,
            priority_weight: task.priority.weight(
                settings.priority_low_weight,
                settings.priority_normal_weight,
                settings.priority_high_weight,
            ),
            deadline_slot: deadline.map(|value| {
                ((value - local_start.with_timezone(&Utc)).num_minutes() / settings.slot_minutes)
                    .max(0) as usize
            }),
            soft_deadline: task.deadline_kind == DeadlineKind::Soft,
            high: task.cognitive_load == CognitiveLoad::High,
            blocked_by: dependency_map.get(&task.id).cloned().unwrap_or_default(),
            slot_minutes: settings.slot_minutes,
            recovery_minutes: settings.recovery_minutes,
            excess_high_penalty: settings.excess_high_penalty,
            feasible,
            cognitive,
            external_fatigue,
            start_slot_idx: None,
        });
    }
    let plan = SolverPlan {
        slots: slots
            .iter()
            .enumerate()
            .map(|(id, _)| SolverSlot { id })
            .collect(),
        tasks: assignments,
        score: None,
        solve_seconds: settings.solve_seconds as u64,
    };
    let solved = solve(plan)?;
    let proposal_id = Uuid::new_v4().to_string();
    let horizon_start = local_start.format(time::STORAGE_FORMAT).to_string();
    let snapshot = serde_json::to_string(&ProposalSnapshot {
        task_versions: tasks
            .iter()
            .map(|task| (task.id.clone(), task.updated_at.clone()))
            .collect(),
        event_versions: db::load_events(conn)
            .map_err(internal)?
            .into_iter()
            .map(|event| (event.id, event.updated_at))
            .collect(),
    })
    .map_err(internal)?;
    let proposal = PlannerProposal {
        id: proposal_id.clone(),
        status: "ready".into(),
        horizon_start,
        horizon_days: settings.horizon_days,
        timezone: timezone_name.clone(),
        score: solved.score.map(|score| score.to_string()),
        created_at: now(),
        applied_at: None,
    };
    conn.execute("INSERT INTO planner_proposals (id,status,horizon_start,horizon_days,timezone,score,snapshot_json,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![proposal.id, proposal.status, proposal.horizon_start, proposal.horizon_days, proposal.timezone, proposal.score, snapshot, proposal.created_at]).map_err(internal)?;
    let mut items = Vec::new();
    for assignment in &solved.tasks {
        let task = &tasks[assignment.id];
        let selected = assignment.start_slot_idx.and_then(|index| slots.get(index));
        let (start_at, end_at, cognitive_penalty) = if let Some(slot) = selected {
            (
                Some(
                    slot.start
                        .with_timezone(&timezone)
                        .format(time::STORAGE_FORMAT)
                        .to_string(),
                ),
                Some(
                    (slot.start + Duration::minutes(task.duration_minutes))
                        .with_timezone(&timezone)
                        .format(time::STORAGE_FORMAT)
                        .to_string(),
                ),
                assignment
                    .cognitive
                    .get(assignment.start_slot_idx.unwrap())
                    .copied()
                    .unwrap_or(0),
            )
        } else {
            (None, None, 0)
        };
        let fatigue_penalty = fatigue_penalty(assignment, &solved.tasks, &slots, &settings);
        let explanation = if selected.is_none() {
            Some("No feasible slot within the configured horizon.".into())
        } else if fatigue_penalty > 0 {
            Some("High cognitive-load streak exceeds the configured recovery policy.".into())
        } else if cognitive_penalty > 0 {
            Some("Scheduled partially outside the cognitive preference window.".into())
        } else {
            None
        };
        let item = PlannerProposalItem {
            id: Uuid::new_v4().to_string(),
            proposal_id: proposal_id.clone(),
            task_id: task.id.clone(),
            start_at,
            end_at,
            scheduled: selected.is_some(),
            cognitive_penalty,
            fatigue_penalty,
            explanation,
        };
        conn.execute("INSERT INTO planner_proposal_items (id,proposal_id,task_id,start_at,end_at,scheduled,cognitive_penalty,fatigue_penalty,explanation) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![item.id,item.proposal_id,item.task_id,item.start_at,item.end_at,item.scheduled as i64,item.cognitive_penalty,item.fatigue_penalty,item.explanation]).map_err(internal)?;
        items.push(item);
    }
    Ok(ProposalDetail { proposal, items })
}

pub fn list_proposals(conn: &Connection) -> Result<Vec<PlannerProposal>, PlannerError> {
    let mut stmt = conn.prepare("SELECT id,status,horizon_start,horizon_days,timezone,score,created_at,applied_at FROM planner_proposals ORDER BY created_at DESC").map_err(internal)?;
    let rows = stmt.query_map([], proposal_from_row).map_err(internal)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)
}

pub fn proposal(conn: &Connection, id: &str) -> Result<ProposalDetail, PlannerError> {
    let proposal = conn.query_row("SELECT id,status,horizon_start,horizon_days,timezone,score,created_at,applied_at FROM planner_proposals WHERE id=?1", [id], proposal_from_row).optional().map_err(internal)?
        .ok_or_else(|| PlannerError::NotFound { resource: "proposal", id: id.into() })?;
    let mut stmt = conn.prepare("SELECT id,proposal_id,task_id,start_at,end_at,scheduled,cognitive_penalty,fatigue_penalty,explanation FROM planner_proposal_items WHERE proposal_id=?1 ORDER BY start_at,task_id").map_err(internal)?;
    let items = stmt
        .query_map([id], proposal_item_from_row)
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;
    Ok(ProposalDetail { proposal, items })
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

fn solve(plan: SolverPlan) -> Result<SolverPlan, PlannerError> {
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

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<PlanningTask> {
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
fn proposal_from_row(row: &Row<'_>) -> rusqlite::Result<PlannerProposal> {
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
fn proposal_item_from_row(row: &Row<'_>) -> rusqlite::Result<PlannerProposalItem> {
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
    })
}
fn settings_from_row(row: &Row<'_>) -> rusqlite::Result<PlannerSettings> {
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
fn priority_db(value: &TaskPriority) -> &'static str {
    match value {
        TaskPriority::Low => "low",
        TaskPriority::Normal => "normal",
        TaskPriority::High => "high",
    }
}
fn priority_from(value: &str) -> TaskPriority {
    match value {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        _ => TaskPriority::Normal,
    }
}
fn cognitive_db(value: &CognitiveLoad) -> &'static str {
    match value {
        CognitiveLoad::Low => "low",
        CognitiveLoad::Medium => "medium",
        CognitiveLoad::High => "high",
    }
}
fn cognitive_from(value: &str) -> CognitiveLoad {
    match value {
        "low" => CognitiveLoad::Low,
        "high" => CognitiveLoad::High,
        _ => CognitiveLoad::Medium,
    }
}
fn deadline_db(value: &DeadlineKind) -> &'static str {
    match value {
        DeadlineKind::None => "none",
        DeadlineKind::Hard => "hard",
        DeadlineKind::Soft => "soft",
    }
}
fn deadline_from(value: &str) -> DeadlineKind {
    match value {
        "hard" => DeadlineKind::Hard,
        "soft" => DeadlineKind::Soft,
        _ => DeadlineKind::None,
    }
}
fn task_state_from(value: &str) -> PlanningTaskState {
    match value {
        "applied" => PlanningTaskState::Applied,
        "missing_event" => PlanningTaskState::MissingEvent,
        _ => PlanningTaskState::Inbox,
    }
}
fn now() -> String {
    Utc::now().format(time::STORAGE_FORMAT).to_string()
}
fn internal(error: impl std::fmt::Display) -> PlannerError {
    PlannerError::Internal(error.to_string())
}
fn validation(error: anyhow::Error) -> PlannerError {
    PlannerError::Validation(error.to_string())
}
fn event_service_error(error: event_service::EventServiceError) -> PlannerError {
    match error {
        event_service::EventServiceError::NotFound { resource, id } => {
            PlannerError::NotFound { resource, id }
        }
        event_service::EventServiceError::Validation(message) => PlannerError::Validation(message),
        event_service::EventServiceError::Conflict(message) => PlannerError::Conflict(message),
        event_service::EventServiceError::Internal(message) => PlannerError::Internal(message),
    }
}
fn require_task(conn: &Connection, id: &str) -> Result<PlanningTask, PlannerError> {
    get_task(conn, id)?.ok_or_else(|| PlannerError::NotFound {
        resource: "task",
        id: id.into(),
    })
}
fn normalize_timezone(value: &str) -> Result<String, PlannerError> {
    time::normalize_timezone(value).map_err(validation)
}
fn normalize_clock(value: &str) -> Result<String, PlannerError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map(|v| v.format("%H:%M").to_string())
        .map_err(|_| PlannerError::Validation(format!("invalid time '{}'; expected HH:MM", value)))
}
fn parse_clock(value: &str) -> Result<NaiveTime, PlannerError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| PlannerError::Validation(format!("invalid time '{}'; expected HH:MM", value)))
}
fn validate_availability(value: &Availability) -> Result<(), PlannerError> {
    if value.0.is_empty() {
        return Err(PlannerError::Validation(
            "configure at least one weekly availability window".into(),
        ));
    }
    for (day, windows) in &value.0 {
        if weekday_index(day).is_none() {
            return Err(PlannerError::Validation(format!(
                "invalid weekday '{}'",
                day
            )));
        }
        if windows.is_empty() {
            return Err(PlannerError::Validation(format!(
                "weekday '{}' has no windows",
                day
            )));
        }
        for window in windows {
            parse_clock(&window.start)?;
            parse_clock(&window.end)?;
        }
    }
    Ok(())
}
fn validate_settings(value: &PlannerSettings) -> Result<(), PlannerError> {
    if !(1..=90).contains(&value.horizon_days) {
        return Err(PlannerError::Validation(
            "horizon_days must be between 1 and 90".into(),
        ));
    }
    if !(5..=120).contains(&value.slot_minutes) {
        return Err(PlannerError::Validation(
            "slot_minutes must be between 5 and 120".into(),
        ));
    }
    if !(1..=120).contains(&value.solve_seconds) {
        return Err(PlannerError::Validation(
            "solve_seconds must be between 1 and 120".into(),
        ));
    }
    if [
        value.priority_low_weight,
        value.priority_normal_weight,
        value.priority_high_weight,
        value.low_outside_penalty,
        value.medium_outside_penalty,
        value.high_outside_penalty,
        value.excess_high_penalty,
    ]
    .iter()
    .any(|v| *v < 0)
    {
        return Err(PlannerError::Validation(
            "planner weights cannot be negative".into(),
        ));
    }
    if value.high_streak_limit < 1 || value.recovery_minutes < 0 {
        return Err(PlannerError::Validation(
            "high streak limit must be positive and recovery minutes cannot be negative".into(),
        ));
    }
    if value.cognitive_enabled {
        for clock in [
            &value.low_window_start,
            &value.low_window_end,
            &value.medium_window_start,
            &value.medium_window_end,
            &value.high_window_start,
            &value.high_window_end,
        ] {
            parse_clock(clock)?;
        }
    }
    Ok(())
}
fn validate_task_input(conn: &Connection, input: &CreateTaskInput) -> Result<(), PlannerError> {
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
fn insert_task(conn: &Connection, task: &PlanningTask) -> Result<(), PlannerError> {
    conn.execute("INSERT INTO planning_tasks (id,title,duration_minutes,target_calendar_id,project_id,priority,cognitive_load,earliest_at,deadline_kind,deadline_at,state,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![task.id,task.title,task.duration_minutes,task.target_calendar_id,task.project_id,priority_db(&task.priority),cognitive_db(&task.cognitive_load),task.earliest_at,deadline_db(&task.deadline_kind),task.deadline_at,match task.state{PlanningTaskState::Inbox=>"inbox",PlanningTaskState::Applied=>"applied",PlanningTaskState::MissingEvent=>"missing_event"},task.created_at,task.updated_at]).map_err(internal)?;
    Ok(())
}
fn dependency_cycle(conn: &Connection) -> Result<bool, PlannerError> {
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
fn require_google_checkpoint(conn: &Connection) -> Result<(), PlannerError> {
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
type UtcInterval = (DateTime<Utc>, DateTime<Utc>);

fn busy_intervals(conn: &Connection) -> Result<Vec<UtcInterval>, PlannerError> {
    Ok(db::load_events(conn)
        .map_err(internal)?
        .into_iter()
        .filter_map(|event| event.start_dt().zip(event.end_dt()))
        .collect())
}
fn applied_high_intervals(conn: &Connection) -> Result<Vec<UtcInterval>, PlannerError> {
    let mut stmt=conn.prepare("SELECT e.start_at,e.end_at,e.timezone FROM planning_tasks t JOIN planning_task_events l ON l.task_id=t.id JOIN events e ON e.id=l.event_id WHERE t.cognitive_load='high' AND e.deleted_at IS NULL").map_err(internal)?;
    let rows = stmt
        .query_map([], |row| {
            let start: String = row.get(0)?;
            let end: String = row.get(1)?;
            let timezone: String = row.get(2)?;
            let start = time::resolve_utc_datetime(&start, &timezone)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let end = time::resolve_utc_datetime(&end, &timezone)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok((start, end))
        })
        .map_err(internal)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)
}
fn dependency_map(
    conn: &Connection,
    tasks: &[PlanningTask],
) -> Result<HashMap<String, Vec<usize>>, PlannerError> {
    let lookup: HashMap<_, _> = tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| (task.id.clone(), idx))
        .collect();
    let mut out = HashMap::new();
    for (from, to) in list_dependencies(conn)? {
        if let (Some(&from), Some(_)) = (lookup.get(&from), lookup.get(&to)) {
            out.entry(to).or_insert_with(Vec::new).push(from);
        }
    }
    Ok(out)
}

fn applied_predecessor_ends(
    conn: &Connection,
    inbox_tasks: &[PlanningTask],
    timezone: &str,
) -> Result<HashMap<String, DateTime<Utc>>, PlannerError> {
    let inbox: std::collections::HashSet<_> =
        inbox_tasks.iter().map(|task| task.id.as_str()).collect();
    let mut ends: HashMap<String, DateTime<Utc>> = HashMap::new();
    for (from, to) in list_dependencies(conn)? {
        if !inbox.contains(to.as_str()) || inbox.contains(from.as_str()) {
            continue;
        }
        let predecessor = require_task(conn, &from)?;
        let event: Option<(String, String)> = conn
            .query_row(
                "SELECT e.end_at,e.timezone FROM planning_task_events l JOIN events e ON e.id=l.event_id WHERE l.task_id=?1 AND e.deleted_at IS NULL",
                [&from],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?;
        let Some((end, event_timezone)) = event else {
            return Err(PlannerError::Conflict(format!(
                "dependency predecessor '{}' is not backed by an active event",
                predecessor.title
            )));
        };
        let end = time::resolve_utc_datetime(&end, &event_timezone)
            .or_else(|_| time::resolve_utc_datetime(&end, timezone))
            .map_err(validation)?;
        ends.entry(to)
            .and_modify(|current| *current = (*current).max(end))
            .or_insert(end);
    }
    Ok(ends)
}
fn make_slots(start: DateTime<Tz>, days: i64, minutes: i64) -> Result<Vec<Slot>, PlannerError> {
    let count = days
        .checked_mul(24)
        .and_then(|v| v.checked_mul(60))
        .and_then(|v| v.checked_div(minutes))
        .ok_or_else(|| PlannerError::Validation("invalid horizon".into()))?;
    Ok((0..count)
        .map(|idx| Slot {
            start: start.with_timezone(&Utc) + Duration::minutes(idx * minutes),
        })
        .collect())
}
struct Slot {
    start: DateTime<Utc>,
}
fn weekday_index(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(0),
        "tue" | "tuesday" => Some(1),
        "wed" | "wednesday" => Some(2),
        "thu" | "thursday" => Some(3),
        "fri" | "friday" => Some(4),
        "sat" | "saturday" => Some(5),
        "sun" | "sunday" => Some(6),
        _ => None,
    }
}
fn available_interval(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    availability: &Availability,
    timezone: Tz,
) -> bool {
    let mut current = start;
    while current < end {
        let local = current.with_timezone(&timezone);
        let key = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
            [local.weekday().num_days_from_monday() as usize];
        let Some(windows) = availability
            .0
            .get(key)
            .or_else(|| availability.0.get(&key.to_uppercase()))
        else {
            return false;
        };
        if !windows
            .iter()
            .any(|window| clock_contains(local.time(), &window.start, &window.end))
        {
            return false;
        }
        current += Duration::minutes(1);
    }
    true
}
fn clock_contains(value: NaiveTime, start: &str, end: &str) -> bool {
    let Ok(start) = NaiveTime::parse_from_str(start, "%H:%M") else {
        return false;
    };
    let Ok(end) = NaiveTime::parse_from_str(end, "%H:%M") else {
        return false;
    };
    if start == end {
        return true;
    }
    if start < end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}
fn overlaps(a: DateTime<Utc>, b: DateTime<Utc>, c: DateTime<Utc>, d: DateTime<Utc>) -> bool {
    a < d && c < b
}
fn cognitive_profile(settings: &PlannerSettings, load: &CognitiveLoad) -> (String, String, i64) {
    match load {
        CognitiveLoad::Low => (
            settings.low_window_start.clone(),
            settings.low_window_end.clone(),
            settings.low_outside_penalty,
        ),
        CognitiveLoad::Medium => (
            settings.medium_window_start.clone(),
            settings.medium_window_end.clone(),
            settings.medium_outside_penalty,
        ),
        CognitiveLoad::High => (
            settings.high_window_start.clone(),
            settings.high_window_end.clone(),
            settings.high_outside_penalty,
        ),
    }
}
fn cognitive_cost(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    window_start: &str,
    window_end: &str,
    penalty: i64,
    timezone: Tz,
) -> i64 {
    let mut cursor = start;
    let mut cost = 0;
    while cursor < end {
        if !clock_contains(
            cursor.with_timezone(&timezone).time(),
            window_start,
            window_end,
        ) {
            cost += penalty;
        }
        cursor += Duration::minutes(1);
    }
    cost
}
fn fatigue_penalty(
    task: &SolverTask,
    all: &[SolverTask],
    slots: &[Slot],
    settings: &PlannerSettings,
) -> i64 {
    if !task.high {
        return 0;
    }
    let Some(start_idx) = task.start_slot_idx else {
        return 0;
    };
    let Some(slot) = slots.get(start_idx) else {
        return 0;
    };
    let start = slot.start;
    let mut current_preceding = 0;
    for other in all {
        if other.id == task.id || !other.high {
            continue;
        }
        if let Some(end) = other.end() {
            let end = slots[0].start + Duration::minutes(end);
            if end <= start && (start - end).num_minutes() < settings.recovery_minutes {
                current_preceding += 1;
            }
        }
    }
    if task.external_fatigue_cost() > 0 || current_preceding >= settings.high_streak_limit as usize
    {
        settings.excess_high_penalty
    } else {
        0
    }
}

fn applied_fatigue_cost(
    high: bool,
    start: DateTime<Utc>,
    applied: &[UtcInterval],
    settings: &PlannerSettings,
) -> i64 {
    if !high {
        return 0;
    }
    let preceding = applied
        .iter()
        .filter(|(_, end)| {
            *end <= start && (start - *end).num_minutes() < settings.recovery_minutes
        })
        .count();
    if preceding >= settings.high_streak_limit as usize {
        settings.excess_high_penalty
    } else {
        0
    }
}
fn validate_snapshot(conn: &Connection, id: &str) -> Result<(), PlannerError> {
    let raw: String = conn
        .query_row(
            "SELECT snapshot_json FROM planner_proposals WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(internal)?;
    let snapshot: ProposalSnapshot = serde_json::from_str(&raw).map_err(internal)?;
    for (task_id, version) in snapshot.task_versions {
        let task = require_task(conn, &task_id)?;
        if task.updated_at != version {
            return Err(PlannerError::Conflict(
                "proposal is stale because an inbox task changed".into(),
            ));
        }
    }
    let events: HashMap<_, _> = db::load_events(conn)
        .map_err(internal)?
        .into_iter()
        .map(|event| (event.id, event.updated_at))
        .collect();
    if events != snapshot.event_versions {
        return Err(PlannerError::Conflict(
            "proposal is stale because calendar availability changed".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn connection() -> (TempDir, Connection, String) {
        let temp = TempDir::new().unwrap();
        let conn = db::open_at(temp.path().join("calendar.db")).unwrap();
        let calendar_id = db::load_calendars(&conn).unwrap()[0].id.clone();
        (temp, conn, calendar_id)
    }

    fn task(calendar_id: String, title: &str) -> CreateTaskInput {
        CreateTaskInput {
            title: title.into(),
            duration_minutes: 60,
            target_calendar_id: calendar_id,
            project_id: None,
            priority: TaskPriority::Normal,
            cognitive_load: CognitiveLoad::Medium,
            earliest_at: None,
            deadline_kind: DeadlineKind::None,
            deadline_at: None,
        }
    }

    #[test]
    fn settings_start_neutral_and_require_availability_for_a_solve() {
        let (_temp, conn, _) = connection();
        let settings = super::settings(&conn).unwrap();
        assert!(!settings.cognitive_enabled);
        assert_eq!(settings.horizon_days, 14);
        assert!(validate_availability(
            &serde_json::from_str::<Availability>(&settings.availability_json).unwrap()
        )
        .is_err());
    }

    #[test]
    fn inbox_task_keeps_cognitive_load_and_deadline_contract() {
        let (_temp, conn, calendar_id) = connection();
        let mut input = task(calendar_id, "Deep work");
        input.cognitive_load = CognitiveLoad::High;
        input.deadline_kind = DeadlineKind::Hard;
        input.deadline_at = Some("2026-09-01 12:00:00".into());
        let created = create_task(&conn, input).unwrap();
        assert_eq!(created.cognitive_load, CognitiveLoad::High);
        assert_eq!(created.state, PlanningTaskState::Inbox);
        assert_eq!(list_tasks(&conn).unwrap().len(), 1);
    }

    #[test]
    fn task_dependencies_reject_cycles() {
        let (_temp, conn, calendar_id) = connection();
        let first = create_task(&conn, task(calendar_id.clone(), "First")).unwrap();
        let second = create_task(&conn, task(calendar_id, "Second")).unwrap();
        add_dependency(&conn, &first.id, &second.id).unwrap();
        assert!(add_dependency(&conn, &second.id, &first.id).is_err());
    }

    #[test]
    fn deleted_linked_event_moves_task_to_missing_event_and_can_return_to_inbox() {
        let (_temp, conn, calendar_id) = connection();
        let created = create_task(&conn, task(calendar_id.clone(), "Recover me")).unwrap();
        let event = event_service::save_event(
            &conn,
            Event::new(
                calendar_id,
                "Recover me",
                "2026-09-01 09:00:00",
                "2026-09-01 10:00:00",
                "UTC",
            ),
            true,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO planner_proposals (id,status,horizon_start,horizon_days,timezone,snapshot_json,created_at) VALUES ('proposal','applied','2026-09-01 00:00:00',1,'UTC','{}','2026-09-01 00:00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO planning_task_events (task_id,event_id,proposal_id) VALUES (?1,?2,'proposal')",
            params![created.id, event.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE planning_tasks SET state='applied' WHERE id=?1",
            [&created.id],
        )
        .unwrap();

        event_service::delete_event(&conn, &event.id).unwrap();
        assert_eq!(
            require_task(&conn, &created.id).unwrap().state,
            PlanningTaskState::MissingEvent
        );

        assert_eq!(
            return_to_inbox(&conn, &created.id).unwrap().state,
            PlanningTaskState::Inbox
        );
        assert!(
            conn.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM planning_task_events WHERE task_id=?1",
                [&created.id],
                |row| row.get(0),
            )
            .unwrap()
                == 0
        );
    }
}
