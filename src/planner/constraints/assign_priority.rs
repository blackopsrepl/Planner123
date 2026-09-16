use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// MEDIUM: schedule as many inbox tasks as possible, weighted by priority.
///
/// Assignment is a preference, not a hard rule, so unschedulable tasks stay
/// unassigned. The weight is the task's priority weight, which is how "prefer
/// high priority" is expressed without adding another score level.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .unassigned()
        .penalize(|task: &SolverTask| HardMediumSoftScore::of_medium(task.priority_weight))
        .named("Assign inbox tasks by priority")
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
