use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task must start at or after the end of every task it depends on.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::tasks()),
            |successor: &SolverTask, predecessor: &SolverTask| {
                successor.index != predecessor.index
                    && successor.depends_on.contains(&predecessor.index)
                    && matches!(
                        (successor.start(), predecessor.end()),
                        (Some(start), Some(end)) if start < end
                    )
            },
        ))
        .penalize(hard_weight(|_: &SolverTask, _: &SolverTask| {
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
