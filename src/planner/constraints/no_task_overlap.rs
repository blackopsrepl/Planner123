use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: two tasks must not occupy overlapping time.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::tasks()),
            |left: &SolverTask, right: &SolverTask| {
                left.index < right.index && left.overlaps_task(right)
            },
        ))
        .penalize(hard_weight(|_: &SolverTask, _: &SolverTask| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("No overlap between inbox tasks")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use solverforge::ConstraintSet;

    fn plan(start_idxs: [Option<usize>; 2]) -> SolverPlan {
        let mut first = task(3, 60);
        first.id = 0;
        first.index = 0;
        first.start_idx = start_idxs[0];
        let mut second = task(3, 60);
        second.id = 1;
        second.task_id = "task-1".into();
        second.index = 1;
        second.start_idx = start_idxs[1];
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![first, second], 1)
    }

    #[test]
    fn penalizes_overlapping_tasks_once() {
        let score = (constraint(),).evaluate_all(&plan([Some(0), Some(1)]));
        assert_eq!(score, HardMediumSoftScore::of_hard(-1));
    }

    #[test]
    fn allows_back_to_back_tasks() {
        let score = (constraint(),).evaluate_all(&plan([Some(0), Some(2)]));
        assert_eq!(score, HardMediumSoftScore::ZERO);
    }
}
