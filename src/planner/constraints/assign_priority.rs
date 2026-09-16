use crate::planner::PlannerError;
use crate::planner_domain::{SolverPlan, SolverTask};
use chrono::{DateTime, Utc};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// MEDIUM: schedule as many inbox tasks as possible, weighted by priority.
///
/// Assignment is a preference, not a hard rule, so unschedulable tasks stay
/// unassigned. The weight is the task's priority weight, which is how "prefer
/// high priority" is expressed without adding another score level. The loader
/// scales each priority point above aggregate possible soft-deadline lateness.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .unassigned()
        .penalize(|task: &SolverTask| HardMediumSoftScore::of_medium(task.priority_weight))
        .named("Assign inbox tasks by priority")
}

/// Encodes assignment/priority above aggregate soft-deadline lateness within
/// the medium score level. One priority point exceeds every possible minute of
/// lateness in this solve, preserving the intended lexicographic order.
pub(crate) fn scale_assignment_penalties(
    tasks: &mut [SolverTask],
    horizon_end: DateTime<Utc>,
) -> Result<(), PlannerError> {
    let max_total_lateness: i128 = tasks
        .iter()
        .filter_map(|task| task.soft_deadline)
        .map(|deadline| i128::from((horizon_end - deadline).num_minutes().max(0)))
        .sum();
    let scale = max_total_lateness + 1;
    let scaled: Vec<i128> = tasks
        .iter()
        .map(|task| i128::from(task.priority_weight) * scale)
        .collect();
    let max_medium_penalty = scaled.iter().sum::<i128>() + max_total_lateness;
    if max_medium_penalty > i128::from(i64::MAX) {
        return Err(PlannerError::Validation(
            "planner priority and deadline weights exceed the supported score range".into(),
        ));
    }
    for (task, penalty) in tasks.iter_mut().zip(scaled) {
        task.priority_weight = penalty as i64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::SolverPlan;
    use solverforge::ConstraintSet;

    fn plan(priority_weight: i64, assigned: bool) -> SolverPlan {
        let mut task = crate::planner_domain::test_support::task(priority_weight, 60);
        task.start_idx = assigned.then_some(0);
        SolverPlan::new(
            crate::planner_domain::test_support::slots(4),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![task],
            1,
        )
    }

    #[test]
    fn penalizes_unassigned_task_by_priority() {
        let score = (constraint(),).evaluate_all(&plan(7, false));
        assert_eq!(score, HardMediumSoftScore::of_medium(-7));
    }

    #[test]
    fn assigned_task_scores_zero() {
        let score = (constraint(),).evaluate_all(&plan(7, true));
        assert_eq!(score, HardMediumSoftScore::ZERO);
    }
}
