use super::support::{task_row, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task must start at or after the end of every task it depends on.
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
                    (left.depends_on.contains(&right.index) && left.start < right.end)
                        || (right.depends_on.contains(&left.index) && right.start < left.end)
                }
                _ => false,
            },
        )
        .penalize(hard_weight(|_: &TimelineRow, _: &TimelineRow| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("Respect task dependencies")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use solverforge::ConstraintSet;

    fn plan(predecessor_slot: Option<usize>, successor_slot: Option<usize>) -> SolverPlan {
        let mut predecessor = task(3, 60);
        predecessor.id = 0;
        predecessor.index = 0;
        predecessor.start_idx = predecessor_slot;
        let mut successor = task(3, 60);
        successor.id = 1;
        successor.task_id = "task-1".into();
        successor.index = 1;
        successor.depends_on = vec![0];
        successor.start_idx = successor_slot;
        SolverPlan::new(
            slots(8),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![predecessor, successor],
            1,
        )
    }

    #[test]
    fn penalizes_successor_starting_before_predecessor_ends() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(1), Some(0))),
            HardMediumSoftScore::of_hard(-1)
        );
    }

    #[test]
    fn allows_successor_after_predecessor() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), Some(2))),
            HardMediumSoftScore::ZERO
        );
    }
}
