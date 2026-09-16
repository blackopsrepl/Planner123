use super::support::{task_row, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: two tasks must not occupy overlapping time.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            SolverPlan::slots(),
            joiner::equal_bi(
                |task: &SolverTask| task.start_idx,
                |slot: &SolverSlot| Some(slot.id),
            ),
        ))
        .project(task_row)
        .join(joiner::equal(|_: &TimelineRow| ()))
        .filter(
            |left: &TimelineRow, right: &TimelineRow| match (left, right) {
                (TimelineRow::Task(left), TimelineRow::Task(right)) => {
                    left.start < right.end && right.start < left.end
                }
                _ => false,
            },
        )
        .penalize(hard_weight(|_: &TimelineRow, _: &TimelineRow| {
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
        SolverPlan::new(
            slots(8),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![first, second],
            1,
        )
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
