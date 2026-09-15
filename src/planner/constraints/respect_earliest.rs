use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task must never start before its earliest start.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| {
            matches!(
                (task.start(), task.earliest_at),
                (Some(start), Some(earliest)) if start < earliest
            )
        })
        .penalize(hard_weight(|_: &SolverTask| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("Respect earliest start")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, earliest_offset_minutes: Option<i64>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        task.earliest_at =
            earliest_offset_minutes.map(|minutes| task.horizon_origin + Duration::minutes(minutes));
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_assignment_before_earliest() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), Some(60))),
            HardMediumSoftScore::of_hard(-1)
        );
    }

    #[test]
    fn ignores_late_or_unbounded_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), Some(60))),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), None)),
            HardMediumSoftScore::ZERO
        );
    }
}
