use super::*;

use super::constraints::{minutes_outside, names, TaskInterval};
use solverforge::Analyzable;

/// Runs one optimization and persists a proposal. Nothing else is mutated.
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

    let inputs = SolverInputs {
        settings: &settings,
        timezone,
        timezone_name: &timezone_name,
        availability: &availability,
        tasks: &tasks,
    };
    let built = build_plan(conn, &inputs)?;
    let horizon_end = built.horizon_end;
    let solved = solve(built.plan)?;
    let penalties = task_penalties(&solved)?;

    let proposal_id = Uuid::new_v4().to_string();
    let horizon_start = solved
        .slots
        .first()
        .map(|slot| {
            slot.start
                .with_timezone(&timezone)
                .format(time::STORAGE_FORMAT)
                .to_string()
        })
        .ok_or_else(|| {
            PlannerError::Validation("the planner horizon produced no candidate slots".into())
        })?;
    let snapshot = proposal_snapshot(conn, &settings)?;
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
    persist_proposal(conn, &proposal, &snapshot)?;

    let mut items = Vec::new();
    for task in &solved.tasks {
        let selected = task.start_idx.and_then(|index| solved.slots.get(index));
        let (start_at, end_at) = match selected {
            Some(slot) => (
                Some(
                    slot.start
                        .with_timezone(&timezone)
                        .format(time::STORAGE_FORMAT)
                        .to_string(),
                ),
                Some(
                    task.end_at(slot)
                        .with_timezone(&timezone)
                        .format(time::STORAGE_FORMAT)
                        .to_string(),
                ),
            ),
            None => (None, None),
        };
        let entry = &penalties[task.index];
        let evidence = if selected.is_some() {
            diagnostics::UnscheduledEvidence {
                outcome: PlannerProposalOutcome::Scheduled,
                busy_blockers: Vec::new(),
                busy_blockers_omitted: 0,
            }
        } else {
            diagnostics::classify(&solved, horizon_end, task.index)
        };
        let explanation = explanation(selected.is_some(), entry, &evidence.outcome);
        let item = PlannerProposalItem {
            id: Uuid::new_v4().to_string(),
            proposal_id: proposal_id.clone(),
            task_id: task.task_id.clone(),
            start_at,
            end_at,
            scheduled: selected.is_some(),
            cognitive_penalty: entry.cognitive,
            fatigue_penalty: entry.fatigue,
            explanation,
            diagnostics: PlannerProposalDiagnostics {
                outcome: evidence.outcome,
                busy_blockers: evidence.busy_blockers,
                busy_blockers_omitted: evidence.busy_blockers_omitted,
            },
        };
        persist_item(conn, &item)?;
        items.push(item);
    }

    Ok(ProposalDetail {
        proposal,
        items,
        applicability: ProposalApplicability::ready(),
    })
}

/// Per-task soft penalties, owned by the task the scoring rule charges:
/// cognitive cost belongs to the task sitting outside its window, and
/// recovery cost belongs to the later task of a violating pair or the task
/// whose applied-block pressure reaches the streak limit.
#[derive(Clone, Default)]
struct TaskPenalties {
    cognitive: i64,
    fatigue: i64,
}

/// Computes penalties for every scheduled task from the same predicates the
/// scoring rules use, then proves the totals reconcile with the solver's own
/// score analysis before anything is persisted.
fn task_penalties(plan: &SolverPlan) -> Result<Vec<TaskPenalties>, PlannerError> {
    let intervals: Vec<Option<TaskInterval>> = plan
        .tasks
        .iter()
        .map(|task| {
            task.start_idx
                .map(|index| TaskInterval::new(task, &plan.slots[index]))
        })
        .collect();

    let mut penalties = vec![TaskPenalties::default(); plan.tasks.len()];
    for (index, interval) in intervals.iter().enumerate() {
        let Some(interval) = interval else { continue };
        let cognitive = plan
            .cognitive_windows
            .iter()
            .find(|window| window.load == plan.tasks[index].load && window.outside_penalty > 0)
            .map_or(0, |window| {
                minutes_outside(interval, window.start, window.end) * window.outside_penalty
            });
        // The recovery rule charges the target per violating predecessor, so
        // item penalties repeat the target's weight for each one.
        let inbox_fatigue: i64 = intervals
            .iter()
            .flatten()
            .filter(|predecessor| {
                predecessor.index != interval.index
                    && interval.gap_recovers(predecessor.end, predecessor.is_high())
            })
            .map(|_| interval.excess_high_penalty)
            .sum();
        let fatigue = if interval.applied_recovery_pressure() >= interval.high_streak_limit as usize
        {
            inbox_fatigue + interval.excess_high_penalty
        } else {
            inbox_fatigue
        };
        penalties[index] = TaskPenalties { cognitive, fatigue };
    }

    reconcile_with_analysis(plan, &penalties)?;
    Ok(penalties)
}

/// Proves per-item penalties are additive with the solver's aggregate score so
/// proposal output can never disagree with the model that produced it.
fn reconcile_with_analysis(
    plan: &SolverPlan,
    penalties: &[TaskPenalties],
) -> Result<(), PlannerError> {
    let analysis = plan.analyze();
    let soft = |name: &str| {
        analysis
            .constraints
            .iter()
            .find(|constraint| constraint.name == name)
            .map_or(0, |constraint| constraint.score.soft())
    };
    let cognitive_total: i64 = penalties.iter().map(|penalty| penalty.cognitive).sum();
    let fatigue_total: i64 = penalties.iter().map(|penalty| penalty.fatigue).sum();
    let expected_cognitive = -soft(names::COGNITIVE_WINDOWS);
    let expected_fatigue = -(soft(names::INBOX_RECOVERY) + soft(names::APPLIED_RECOVERY));
    if cognitive_total != expected_cognitive || fatigue_total != expected_fatigue {
        return Err(PlannerError::Internal(format!(
            "proposal penalties do not reconcile with score analysis: cognitive {cognitive_total} vs {expected_cognitive}, fatigue {fatigue_total} vs {expected_fatigue}"
        )));
    }
    Ok(())
}

fn explanation(
    scheduled: bool,
    entry: &TaskPenalties,
    outcome: &PlannerProposalOutcome,
) -> Option<String> {
    if !scheduled {
        return match outcome {
            PlannerProposalOutcome::NoHardFeasibleSlot => {
                Some("No hard-feasible slot within the configured horizon.".into())
            }
            PlannerProposalOutcome::FeasibleButNotSelected => Some(
                "Hard-feasible slots exist, but the optimizer could not select one with the other tasks."
                    .into(),
            ),
            _ => Some("Not scheduled within the configured horizon.".into()),
        };
    }
    if entry.fatigue > 0 {
        return Some("High cognitive-load recovery gap below the configured minimum.".into());
    }
    if entry.cognitive > 0 {
        return Some("Scheduled partially outside the cognitive preference window.".into());
    }
    None
}

fn proposal_snapshot(
    conn: &Connection,
    settings: &PlannerSettings,
) -> Result<String, PlannerError> {
    serde_json::to_string(&ProposalSnapshot {
        task_versions: list_tasks(conn)?
            .into_iter()
            .map(|task| (task.id, task.updated_at))
            .collect(),
        event_versions: db::load_events(conn)
            .map_err(internal)?
            .into_iter()
            .map(|event| (event.id, event.updated_at))
            .collect(),
        settings_json: Some(canonical_settings_snapshot(settings)?),
        dependencies: Some(list_dependencies(conn)?),
    })
    .map_err(internal)
}

fn persist_proposal(
    conn: &Connection,
    proposal: &PlannerProposal,
    snapshot: &str,
) -> Result<(), PlannerError> {
    conn.execute(
        "INSERT INTO planner_proposals (id,status,horizon_start,horizon_days,timezone,score,snapshot_json,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![proposal.id, proposal.status, proposal.horizon_start, proposal.horizon_days, proposal.timezone, proposal.score, snapshot, proposal.created_at],
    )
    .map_err(internal)?;
    Ok(())
}

fn persist_item(conn: &Connection, item: &PlannerProposalItem) -> Result<(), PlannerError> {
    let diagnostics_json = serde_json::to_string(&item.diagnostics).map_err(internal)?;
    conn.execute(
        "INSERT INTO planner_proposal_items (id,proposal_id,task_id,start_at,end_at,scheduled,cognitive_penalty,fatigue_penalty,explanation,diagnostics_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![item.id,item.proposal_id,item.task_id,item.start_at,item.end_at,item.scheduled as i64,item.cognitive_penalty,item.fatigue_penalty,item.explanation,diagnostics_json],
    )
    .map_err(internal)?;
    Ok(())
}
