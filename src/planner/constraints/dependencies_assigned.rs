use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

#[derive(Clone)]
struct AssignmentState {
    index: usize,
    assigned: bool,
    depends_on: Vec<usize>,
}

/// HARD: an assigned successor requires every inbox predecessor to be assigned.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .group_by(
            |_: &SolverTask| (),
            collect_vec(|task: &SolverTask| AssignmentState {
                index: task.index,
                assigned: task.start_idx.is_some(),
                depends_on: task.depends_on.clone(),
            }),
        )
        .penalize(hard_weight(
            |_: &(), tasks: &CollectedVec<AssignmentState>| {
                let assigned: Vec<_> = tasks
                    .iter()
                    .map(|task| (task.index, task.assigned))
                    .collect();
                let violations = tasks
                    .iter()
                    .filter(|task| task.assigned)
                    .flat_map(|task| &task.depends_on)
                    .filter(|predecessor| {
                        !assigned
                            .iter()
                            .any(|(index, is_assigned)| index == *predecessor && *is_assigned)
                    })
                    .count();
                HardMediumSoftScore::of_hard(violations as i64)
            },
        ))
        .named("Assign task dependency predecessors")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use solverforge::ConstraintSet;

    fn plan(predecessor_slot: Option<usize>, successor_slot: Option<usize>) -> SolverPlan {
        let mut predecessor = task(3, 60);
        predecessor.start_idx = predecessor_slot;
        let mut successor = task(3, 60);
        successor.id = 1;
        successor.index = 1;
        successor.task_id = "task-1".into();
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
    fn rejects_an_assigned_successor_with_an_unassigned_predecessor() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None, Some(0))),
            HardMediumSoftScore::of_hard(-1)
        );
    }

    #[test]
    fn allows_both_assigned_or_both_unassigned() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), Some(2))),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None, None)),
            HardMediumSoftScore::ZERO
        );
    }
}
