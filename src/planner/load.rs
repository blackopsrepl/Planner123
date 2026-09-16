//! Pure translation of persisted planner state into SolverForge facts and
//! entities.
//!
//! This module contains no scheduling policy: it builds the uniform slot grid,
//! canonical disjoint availability windows, cognitive windows, blocked
//! intervals, and task entities. Every rule that decides legality or quality
//! lives in `crate::planner::constraints`.

use super::*;

/// Everything the loader needs from the database, already validated.
pub(super) struct SolverInputs<'a> {
    pub settings: &'a PlannerSettings,
    pub timezone: Tz,
    pub timezone_name: &'a str,
    pub availability: &'a Availability,
    pub tasks: &'a [PlanningTask],
}

/// The planning solution plus the horizon bound it was built against.
pub(super) struct BuiltPlan {
    pub plan: SolverPlan,
    pub horizon_end: DateTime<Utc>,
}

/// Builds the planning solution from persisted state.
pub(super) fn build_plan(
    conn: &Connection,
    inputs: &SolverInputs<'_>,
) -> Result<BuiltPlan, PlannerError> {
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
    let busy = busy_occurrences(conn, origin, horizon_end)?
        .into_iter()
        .enumerate()
        .map(|(index, occurrence)| SolverBusy {
            id: format!("busy:{index}:{}", occurrence.event_id),
            event_id: occurrence.event_id,
            event_title: occurrence.event_title,
            calendar_id: occurrence.calendar_id,
            recurring: occurrence.recurring,
            start: occurrence.start,
            end: occurrence.end,
        })
        .collect();
    let applied = applied_blocks(conn, inputs.tasks, &dependencies)?;
    let applied_ends = applied_recovery_ends(&applied);

    Ok(BuiltPlan {
        plan: SolverPlan::new(
            slots,
            busy,
            applied,
            availability_facts(inputs.availability)?,
            cognitive_facts(settings)?,
            build_tasks(inputs, now, &dependency_map, &applied_ends)?,
            settings.solve_seconds as u64,
        ),
        horizon_end,
    })
}

fn applied_recovery_ends(applied: &[SolverAppliedBlock]) -> Vec<DateTime<Utc>> {
    applied
        .iter()
        .filter(|block| block.high)
        .map(|block| block.end)
        .collect()
}

fn build_tasks(
    inputs: &SolverInputs<'_>,
    now: DateTime<Utc>,
    dependency_map: &HashMap<String, Vec<usize>>,
    applied_ends: &[DateTime<Utc>],
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
                applied_predecessor_ends: applied_ends.to_vec(),
                not_before: now,
                timezone: inputs.timezone,
                recovery_minutes: settings.recovery_minutes,
                excess_high_penalty: settings.excess_high_penalty,
                high_streak_limit: settings.high_streak_limit,
                start_idx: None,
            })
        })
        .collect()
}

/// Canonicalizes weekly windows into disjoint per-weekday intervals. The
/// canonical form keeps the coverage rule exact: a minute can be contained by
/// at most one fact, so overlapping user windows never double-count.
fn availability_facts(
    availability: &Availability,
) -> Result<Vec<SolverAvailability>, PlannerError> {
    let mut by_weekday: BTreeMap<u32, Vec<(NaiveTime, NaiveTime)>> = BTreeMap::new();
    for (day, windows) in &availability.0 {
        let weekday = weekday_index(day)
            .ok_or_else(|| PlannerError::Validation(format!("invalid weekday '{day}'")))?;
        for window in windows {
            let start = parse_clock(&window.start)?;
            let end = parse_clock(&window.end)?;
            by_weekday.entry(weekday).or_default().push((start, end));
        }
    }
    let mut facts = Vec::new();
    for (weekday, mut windows) in by_weekday {
        windows.sort();
        let mut merged: Vec<(NaiveTime, NaiveTime)> = Vec::new();
        for (start, end) in windows {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        for (start, end) in merged {
            facts.push(SolverAvailability {
                id: format!("availability:{weekday}:{}", facts.len()),
                weekday,
                start,
                end,
            });
        }
    }
    Ok(facts)
}

fn cognitive_facts(settings: &PlannerSettings) -> Result<Vec<SolverCognitiveWindow>, PlannerError> {
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
        .map(|(load, start, end, penalty)| {
            Ok(SolverCognitiveWindow {
                id: format!("cognitive:{load}"),
                load,
                start: parse_clock(start)?,
                end: parse_clock(end)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::applied_block;

    fn window(start: &str, end: &str) -> TimeWindow {
        TimeWindow {
            start: start.into(),
            end: end.into(),
        }
    }

    #[test]
    fn availability_overlaps_merge_into_one_disjoint_fact() {
        let availability = Availability(BTreeMap::from([(
            "mon".into(),
            vec![window("09:00", "12:00"), window("10:00", "13:00")],
        )]));
        let facts = availability_facts(&availability).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].start, parse_clock("09:00").unwrap());
        assert_eq!(facts[0].end, parse_clock("13:00").unwrap());
    }

    #[test]
    fn availability_keeps_split_shifts_separate() {
        let availability = Availability(BTreeMap::from([(
            "mon".into(),
            vec![window("09:00", "12:00"), window("13:00", "17:00")],
        )]));
        assert_eq!(availability_facts(&availability).unwrap().len(), 2);
    }

    #[test]
    fn recovery_uses_all_applied_high_load_blocks_without_dependency_edges() {
        let high = applied_block(0, 60, true, vec![]);
        let low = applied_block(60, 120, false, vec![0]);
        let expected = high.end;
        assert_eq!(applied_recovery_ends(&[high, low]), vec![expected]);
    }
}
