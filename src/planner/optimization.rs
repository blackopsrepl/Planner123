use super::*;

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
        PlannerError::Validation(
            "configure a planner timezone before optimizing; use an IANA name such as Europe/Rome or UTC"
                .into(),
        )
    })?;
    let timezone = Tz::from_str(&timezone_name).map_err(|_| {
        PlannerError::Validation(
            "planner timezone must be an IANA name such as Europe/Rome or UTC".into(),
        )
    })?;
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
    let horizon_end = local_horizon_end(local_start, settings.horizon_days)?;
    let slots = make_slots(
        local_start.with_timezone(&Utc),
        horizon_end,
        settings.slot_minutes,
    )?;
    let busy = busy_intervals(conn, local_start.with_timezone(&Utc), horizon_end)?;
    let dependency_map = dependency_map(conn, &tasks)?;
    let applied_predecessor_ends = applied_predecessor_ends(conn, &tasks, &timezone_name)?;
    let applied_high = applied_high_intervals(conn)?;
    let mut assignments = Vec::new();
    let mut busy_evidence = Vec::new();
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
        let mut task_busy_evidence = BTreeSet::new();
        for slot in &slots {
            let end = slot.start + Duration::minutes(task.duration_minutes);
            let hard_feasible_without_busy =
                available_interval(slot.start, end, &availability, timezone)
                    && end <= horizon_end
                    && earliest.map(|value| slot.start >= value).unwrap_or(true)
                    && applied_predecessor_ends
                        .get(&task.id)
                        .map(|value| slot.start >= *value)
                        .unwrap_or(true)
                    && (task.deadline_kind != DeadlineKind::Hard
                        || deadline.map(|value| end <= value).unwrap_or(false));
            let mut has_overlapping_busy_time = false;
            for (index, occurrence) in busy.iter().enumerate() {
                if overlaps(slot.start, end, occurrence.start, occurrence.end) {
                    has_overlapping_busy_time = true;
                    if hard_feasible_without_busy {
                        task_busy_evidence.insert(index);
                    }
                }
            }
            let allowed = hard_feasible_without_busy && !has_overlapping_busy_time;
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
        busy_evidence.push(task_busy_evidence.into_iter().collect::<Vec<_>>());
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
        settings_json: Some(canonical_settings_snapshot(&settings)?),
        dependencies: Some(list_dependencies(conn)?),
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
        let has_hard_feasible_slot = assignment.feasible.iter().any(|allowed| *allowed);
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
        let diagnostics = if selected.is_none() && !has_hard_feasible_slot {
            let evidence = &busy_evidence[assignment.id];
            let busy_blockers = evidence
                .iter()
                .take(5)
                .map(|index| busy_blocker(&busy[*index], timezone))
                .collect();
            PlannerProposalDiagnostics {
                outcome: PlannerProposalOutcome::NoHardFeasibleSlot,
                busy_blockers,
                busy_blockers_omitted: evidence.len().saturating_sub(5),
            }
        } else if selected.is_none() {
            PlannerProposalDiagnostics {
                outcome: PlannerProposalOutcome::FeasibleButNotSelected,
                ..Default::default()
            }
        } else {
            PlannerProposalDiagnostics::default()
        };
        let explanation = if selected.is_none() && !has_hard_feasible_slot {
            Some("No hard-feasible slot within the configured horizon.".into())
        } else if selected.is_none() {
            Some("Hard-feasible slots exist, but the optimizer could not select one with the other tasks.".into())
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
            diagnostics,
        };
        let diagnostics_json = serde_json::to_string(&item.diagnostics).map_err(internal)?;
        conn.execute("INSERT INTO planner_proposal_items (id,proposal_id,task_id,start_at,end_at,scheduled,cognitive_penalty,fatigue_penalty,explanation,diagnostics_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![item.id,item.proposal_id,item.task_id,item.start_at,item.end_at,item.scheduled as i64,item.cognitive_penalty,item.fatigue_penalty,item.explanation,diagnostics_json]).map_err(internal)?;
        items.push(item);
    }
    Ok(ProposalDetail {
        proposal,
        items,
        applicability: ProposalApplicability::ready(),
    })
}
