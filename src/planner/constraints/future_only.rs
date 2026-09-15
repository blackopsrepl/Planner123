use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: no task may be assigned before the optimization instant.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.start().is_some_and(|start| start < task.not_before))
        .penalize(hard_weight(|_: &SolverTask| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("Never schedule in the past")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, not_before_offset_minutes: i64) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        task.not_before = task.horizon_origin + Duration::minutes(not_before_offset_minutes);
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_assignment_before_the_optimization_instant() {
        let score = (constraint(),).evaluate_all(&plan(Some(0), 60));
        assert_eq!(score, HardMediumSoftScore::of_hard(-1));
    }

    #[test]
    fn ignores_future_or_unassigned_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), 0)),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None, 60)),
            HardMediumSoftScore::ZERO
        );
    }
}
