use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize high cognitive-load tasks that are not separated by the
/// configured recovery gap.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::tasks()),
            |left: &SolverTask, right: &SolverTask| {
                left.index < right.index
                    && left.is_high()
                    && right.is_high()
                    && matches!(left.gap_between(right), Some(gap) if gap < left.recovery_minutes)
            },
        ))
        .penalize(|left: &SolverTask, _right: &SolverTask| {
            HardMediumSoftScore::of_soft(left.excess_high_penalty)
        })
        .named("High cognitive-load recovery")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use solverforge::ConstraintSet;

    fn plan(loads: [usize; 2], start_idxs: [Option<usize>; 2]) -> SolverPlan {
        let mut first = task(3, 60);
        first.id = 0;
        first.index = 0;
        first.load = loads[0];
        first.start_idx = start_idxs[0];
        let mut second = task(3, 60);
        second.id = 1;
        second.task_id = "task-1".into();
        second.index = 1;
        second.load = loads[1];
        second.start_idx = start_idxs[1];
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![first, second], 1)
    }

    #[test]
    fn penalizes_high_load_tasks_without_recovery() {
        let score = (constraint(),).evaluate_all(&plan([2, 2], [Some(0), Some(2)]));
        assert_eq!(score, HardMediumSoftScore::of_soft(-5));
    }

    #[test]
    fn ignores_non_high_or_separated_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan([1, 2], [Some(0), Some(2)])),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan([2, 2], [Some(0), Some(4)])),
            HardMediumSoftScore::ZERO
        );
    }
}
