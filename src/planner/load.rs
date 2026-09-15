//! Pure translation of persisted planner state into SolverForge facts and
//! entities.
//!
//! This module contains no scheduling policy: it builds the uniform slot grid,
//! the raw availability and cognitive windows, the blocked intervals, and the
//! task entities. Every rule that decides legality or quality lives in
//! `crate::planner::constraints`.

use super::*;

/// Everything the loader needs from the database, already validated.
pub(super) struct SolverInputs<'a> {
    pub settings: &'a PlannerSettings,
    pub timezone: Tz,
    pub timezone_name: &'a str,
    pub availability: &'a Availability,
    pub tasks: &'a [PlanningTask],
}

/// Builds the planning solution from persisted state.
pub(super) fn build_plan(
    conn: &Connection,
    inputs: &SolverInputs<'_>,
) -> Result<SolverPlan, PlannerError> {
    let settings = inputs.settings;
    let now = Utc::now();
    let local_now = now.with_timezone(&inputs.timezone);
    let local_start = inputs
        .timezone
        .from_local_datetime(&local_now.date_naive().and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or_else(|| {
            PlannerError::Validation("planner horizon starts at an invalid local time".into())
        })?;
    let horizon_end = local_horizon_end(local_start, settings.horizon_days)?;
    let slots = make_slots(
        local_start.with_timezone(&Utc),
        horizon_end,
        settings.slot_minutes,
    )?;
    let origin = slots
        .first()
        .map(|slot| slot.start)
        .unwrap_or_else(|| local_start.with_timezone(&Utc));

    let dependencies = list_dependencies(conn)?;
    let dependency_map = dependency_map(conn, inputs.tasks)?;
    let mut busy = existing_busy(conn, origin, horizon_end)?;
    busy.extend(applied_busy(conn, inputs.tasks, &dependencies)?);

    Ok(SolverPlan::new(
        slots,
        busy,
        availability_facts(inputs.availability),
        cognitive_facts(settings),
        build_tasks(inputs, origin, now, &dependency_map)?,
        settings.solve_seconds.max(1) as u64,
    ))
}

fn existing_busy(
    conn: &Connection,
    horizon_start: DateTime<Utc>,
    horizon_end: DateTime<Utc>,
) -> Result<Vec<SolverBusy>, PlannerError> {
    Ok(busy_intervals(conn, horizon_start, horizon_end)?
        .into_iter()
        .enumerate()
        .map(|(index, occurrence)| SolverBusy {
            id: format!("busy:{index}"),
            index,
            start: occurrence.start,
            end: occurrence.end,
            high: false,
            successors: Vec::new(),
            event_id: occurrence.event_id,
            event_title: occurrence.event_title,
            calendar_id: occurrence.calendar_id,
            recurring: occurrence.recurring,
        })
        .collect())
}

fn build_tasks(
    inputs: &SolverInputs<'_>,
    origin: DateTime<Utc>,
    now: DateTime<Utc>,
    dependency_map: &HashMap<String, Vec<usize>>,
) -> Result<Vec<SolverTask>, PlannerError> {
    let settings = inputs.settings;
    inputs
        .tasks
        .iter()
        .enumerate()
        .map(|(index, task)| {
            let earliest = task
                .earliest_at
                .as_deref()
                .map(|value| time::resolve_utc_datetime(value, inputs.timezone_name))
                .transpose()
                .map_err(validation)?;
            let deadline = task
                .deadline_at
                .as_deref()
                .map(|value| time::resolve_utc_datetime(value, inputs.timezone_name))
                .transpose()
                .map_err(validation)?;
            Ok(SolverTask {
                id: index,
                task_id: task.id.clone(),
                index,
                duration_minutes: task.duration_minutes,
                priority_weight: task.priority.weight(
                    settings.priority_low_weight,
                    settings.priority_normal_weight,
                    settings.priority_high_weight,
                ),
                load: load_key(&task.cognitive_load),
                earliest_at: earliest,
                hard_deadline: (task.deadline_kind == DeadlineKind::Hard)
                    .then_some(deadline)
                    .flatten(),
                soft_deadline: (task.deadline_kind == DeadlineKind::Soft)
                    .then_some(deadline)
                    .flatten(),
                depends_on: dependency_map.get(&task.id).cloned().unwrap_or_default(),
                not_before: now,
                timezone: inputs.timezone,
                slot_minutes: settings.slot_minutes,
                horizon_origin: origin,
                recovery_minutes: settings.recovery_minutes,
                excess_high_penalty: settings.excess_high_penalty,
                start_idx: None,
            })
        })
        .collect()
}

fn availability_facts(availability: &Availability) -> Vec<SolverAvailability> {
    let mut facts = Vec::new();
    for (day, windows) in &availability.0 {
        let Some(weekday) = weekday_index(day) else {
            continue;
        };
        for window in windows {
            let (Ok(start), Ok(end)) = (parse_clock(&window.start), parse_clock(&window.end))
            else {
                continue;
            };
            facts.push(SolverAvailability {
                id: format!("availability:{weekday}:{}", facts.len()),
                index: facts.len(),
                weekday,
                start,
                end,
            });
        }
    }
    facts
}

fn cognitive_facts(settings: &PlannerSettings) -> Vec<SolverCognitiveWindow> {
    let levels = [
        (
            0,
            &settings.low_window_start,
            &settings.low_window_end,
            settings.low_outside_penalty,
        ),
        (
            1,
            &settings.medium_window_start,
            &settings.medium_window_end,
            settings.medium_outside_penalty,
        ),
        (
            2,
            &settings.high_window_start,
            &settings.high_window_end,
            settings.high_outside_penalty,
        ),
    ];
    levels
        .into_iter()
        .filter_map(|(load, start, end, penalty)| {
            let (Ok(start), Ok(end)) = (parse_clock(start), parse_clock(end)) else {
                return None;
            };
            Some(SolverCognitiveWindow {
                id: format!("cognitive:{load}"),
                load,
                start,
                end,
                outside_penalty: if settings.cognitive_enabled {
                    penalty
                } else {
                    0
                },
            })
        })
        .collect()
}

fn load_key(load: &CognitiveLoad) -> usize {
    match load {
        CognitiveLoad::Low => 0,
        CognitiveLoad::Medium => 1,
        CognitiveLoad::High => 2,
    }
}

fn parse_clock(value: &str) -> Result<NaiveTime, ()> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| ())
}
