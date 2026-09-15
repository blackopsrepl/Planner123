use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task must end at or before its hard deadline.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| {
            matches!(
                (task.end(), task.hard_deadline),
                (Some(end), Some(deadline)) if end > deadline
            )
        })
        .penalize(hard_weight(|_: &SolverTask| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("Respect hard deadline")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, deadline_offset_minutes: Option<i64>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        task.hard_deadline =
            deadline_offset_minutes.map(|minutes| task.horizon_origin + Duration::minutes(minutes));
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_missing_a_hard_deadline() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), Some(60))),
            HardMediumSoftScore::of_hard(-1)
        );
    }

    #[test]
    fn ignores_met_or_absent_deadlines() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(1), Some(120))),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), None)),
            HardMediumSoftScore::ZERO
        );
    }
}
