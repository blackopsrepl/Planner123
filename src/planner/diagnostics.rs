//! Proposal evidence for tasks the solver left unassigned.
//!
//! Classification asks a per-task question: could this task, on its own,
//! occupy any candidate slot without breaking a hard rule? It deliberately
//! ignores where other inbox tasks landed, because "not selected" and
//! "impossible" are different outcomes with different remediation.

use super::*;

use super::constraints::{fully_covered, TaskInterval};
use crate::models::{PlannerBusyBlocker, PlannerProposalOutcome};

/// Upper bound on persisted blocker evidence per proposal item.
const MAX_BLOCKERS: usize = 5;

pub(super) struct UnscheduledEvidence {
    pub outcome: PlannerProposalOutcome,
    pub busy_blockers: Vec<PlannerBusyBlocker>,
    pub busy_blockers_omitted: usize,
}

/// Classifies one unassigned task and collects the calendar occurrences that
/// removed its otherwise hard-feasible candidate slots.
pub(super) fn classify(
    plan: &SolverPlan,
    horizon_end: DateTime<Utc>,
    task_index: usize,
) -> UnscheduledEvidence {
    let task = &plan.tasks[task_index];
    let mut visiting = vec![false; plan.tasks.len()];
    if task.depends_on.iter().any(|predecessor| {
        !dependency_chain_has_hard_slot(plan, horizon_end, *predecessor, &mut visiting)
    }) {
        return UnscheduledEvidence {
            outcome: PlannerProposalOutcome::NoHardFeasibleSlot,
            busy_blockers: Vec::new(),
            busy_blockers_omitted: 0,
        };
    }
    let mut blocked_by: Vec<&SolverBusy> = Vec::new();
    for slot in &plan.slots {
        let interval = TaskInterval::new(task, slot);
        if !hard_feasible_ignoring_busy(plan, horizon_end, task_index, &interval) {
            continue;
        }
        let overlapping: Vec<&SolverBusy> = plan
            .busy
            .iter()
            .filter(|busy| interval.overlaps(busy.start, busy.end))
            .collect();
        if overlapping.is_empty() {
            return UnscheduledEvidence {
                outcome: PlannerProposalOutcome::FeasibleButNotSelected,
                busy_blockers: Vec::new(),
                busy_blockers_omitted: 0,
            };
        }
        blocked_by.extend(overlapping);
    }

    blocked_by.sort_by(|left, right| {
        (&left.start, &left.end, &left.calendar_id, &left.event_id).cmp(&(
            &right.start,
            &right.end,
            &right.calendar_id,
            &right.event_id,
        ))
    });
    blocked_by.dedup_by(|left, right| {
        left.event_id == right.event_id && left.start == right.start && left.end == right.end
    });
    let busy_blockers_omitted = blocked_by.len().saturating_sub(MAX_BLOCKERS);
    UnscheduledEvidence {
        outcome: PlannerProposalOutcome::NoHardFeasibleSlot,
        busy_blockers: blocked_by
            .into_iter()
            .take(MAX_BLOCKERS)
            .map(|occurrence| busy_blocker(occurrence, task.timezone))
            .collect(),
        busy_blockers_omitted,
    }
}

fn has_individual_hard_slot(
    plan: &SolverPlan,
    horizon_end: DateTime<Utc>,
    task_index: usize,
) -> bool {
    let task = &plan.tasks[task_index];
    plan.slots.iter().any(|slot| {
        let interval = TaskInterval::new(task, slot);
        hard_feasible_ignoring_busy(plan, horizon_end, task_index, &interval)
            && plan
                .busy
                .iter()
                .all(|busy| !interval.overlaps(busy.start, busy.end))
    })
}

fn dependency_chain_has_hard_slot(
    plan: &SolverPlan,
    horizon_end: DateTime<Utc>,
    task_index: usize,
    visiting: &mut [bool],
) -> bool {
    let Some(task) = plan.tasks.get(task_index) else {
        return false;
    };
    if visiting[task_index] || !has_individual_hard_slot(plan, horizon_end, task_index) {
        return false;
    }
    visiting[task_index] = true;
    let feasible = task.depends_on.iter().all(|predecessor| {
        dependency_chain_has_hard_slot(plan, horizon_end, *predecessor, visiting)
    });
    visiting[task_index] = false;
    feasible
}

/// Hard rules from the constraint set, minus busy overlap and minus the
/// placement of other inbox tasks: those two are what separate "impossible"
/// from "not selected".
fn hard_feasible_ignoring_busy(
    plan: &SolverPlan,
    horizon_end: DateTime<Utc>,
    task_index: usize,
    interval: &TaskInterval,
) -> bool {
    let task = &plan.tasks[task_index];
    interval.start >= task.not_before
        && task
            .earliest_at
            .is_none_or(|earliest| interval.start >= earliest)
        && task
            .hard_deadline
            .is_none_or(|deadline| interval.end <= deadline)
        && plan
            .applied_blocks
            .iter()
            .filter(|block| block.successors.contains(&task_index))
            .all(|block| interval.start >= block.end)
        && interval.end <= horizon_end
        && fully_covered(interval, &plan.availability)
}
