use super::*;

use std::collections::BTreeMap;

use solverforge::{Analyzable, HardMediumSoftScore, ScoreAnalysis};

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
    let solved = solve(build_plan(conn, &inputs)?)?;
    let penalties = task_penalties(&solved);

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
        .unwrap_or_else(now);
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
                    (slot.start + Duration::minutes(task.duration_minutes))
                        .with_timezone(&timezone)
                        .format(time::STORAGE_FORMAT)
                        .to_string(),
                ),
            ),
            None => (None, None),
        };
        let entry = penalties.get(&task.index).cloned().unwrap_or_default();
        let item = PlannerProposalItem {
            id: Uuid::new_v4().to_string(),
            proposal_id: proposal_id.clone(),
            task_id: task.task_id.clone(),
            start_at,
            end_at,
            scheduled: selected.is_some(),
            cognitive_penalty: entry.cognitive,
            fatigue_penalty: entry.fatigue,
            explanation: explanation(selected.is_some(), &entry),
            diagnostics: PlannerProposalDiagnostics {
                outcome: if selected.is_some() {
                    PlannerProposalOutcome::Scheduled
                } else {
                    PlannerProposalOutcome::Unassigned
                },
                busy_blockers: Vec::new(),
                busy_blockers_omitted: 0,
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

/// Per-task soft penalties recovered from framework score analysis.
#[derive(Clone, Default)]
struct TaskPenalties {
    cognitive: i64,
    fatigue: i64,
}

fn task_penalties(plan: &SolverPlan) -> BTreeMap<usize, TaskPenalties> {
    let baseline = plan.analyze();
    let mut map: BTreeMap<usize, TaskPenalties> = BTreeMap::new();
    for task in plan.tasks.iter().filter(|task| task.start_idx.is_some()) {
        let mut without = plan.clone();
        without.tasks[task.index].start_idx = None;
        let counterfactual = without.analyze();
        let cognitive = contribution(&baseline, &counterfactual, "Prefer cognitive windows");
        let fatigue = contribution(&baseline, &counterfactual, "High cognitive-load recovery")
            + contribution(
                &baseline,
                &counterfactual,
                "Applied high cognitive-load recovery",
            );
        if cognitive > 0 || fatigue > 0 {
            map.insert(task.index, TaskPenalties { cognitive, fatigue });
        }
    }
    map
}

fn contribution(
    baseline: &ScoreAnalysis<HardMediumSoftScore>,
    counterfactual: &ScoreAnalysis<HardMediumSoftScore>,
    name: &str,
) -> i64 {
    let soft = |analysis: &ScoreAnalysis<HardMediumSoftScore>| {
        analysis
            .constraints
            .iter()
            .find(|constraint| constraint.name == name)
            .map_or(0, |constraint| constraint.score.soft())
    };
    (soft(counterfactual) - soft(baseline)).max(0)
}

fn explanation(scheduled: bool, entry: &TaskPenalties) -> Option<String> {
    if !scheduled {
        return Some("Not scheduled: left unassigned within the configured horizon.".into());
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
