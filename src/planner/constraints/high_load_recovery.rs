use super::support::{task_row, TaskInterval, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: separate high cognitive-load inbox tasks by the configured recovery
/// gap. The later task of a violating pair carries the penalty; work that
/// starts after a task never creates pressure on it.
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
        .filter(|left: &TimelineRow, right: &TimelineRow| violation_penalty(left, right) > 0)
        .penalize(|left: &TimelineRow, right: &TimelineRow| {
            HardMediumSoftScore::of_soft(violation_penalty(left, right))
        })
        .named("High cognitive-load recovery")
}

/// The penalty the later task of a violating pair owes, or zero when the pair
/// is separated enough or not both high-load.
fn violation_penalty(left: &TimelineRow, right: &TimelineRow) -> i64 {
    match (left, right) {
        (TimelineRow::Task(predecessor), TimelineRow::Task(target)) => {
            if precedes(predecessor, target) {
                target.excess_high_penalty
            } else if precedes(target, predecessor) {
                predecessor.excess_high_penalty
            } else {
                0
            }
        }
        _ => 0,
    }
}

fn precedes(predecessor: &TaskInterval, target: &TaskInterval) -> bool {
    predecessor.is_high()
        && target.is_high()
        && predecessor.end <= target.start
        && (target.start - predecessor.end).num_minutes() < target.recovery_minutes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};

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

    fn high_load(mut plan: SolverPlan) -> SolverPlan {
        for task in &mut plan.tasks {
            task.load = 2;
            task.recovery_minutes = 60;
        }
        plan
    }

    #[test]
    fn penalizes_the_later_task_of_a_back_to_back_pair() {
        let plan = high_load(plan([Some(0), Some(2)]));
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::of_soft(-5)
        );
    }

    #[test]
    fn separated_or_low_load_pairs_are_ignored() {
        let separated = high_load(plan([Some(0), Some(4)]));
        assert_eq!(
            (constraint(),).evaluate_all(&separated),
            HardMediumSoftScore::ZERO
        );
        let low_pair = plan([Some(0), Some(2)]);
        assert_eq!(
            (constraint(),).evaluate_all(&low_pair),
            HardMediumSoftScore::ZERO
        );
    }

    #[test]
    fn unassigned_predecessors_create_no_pressure() {
        let half_assigned = high_load(plan([None, Some(2)]));
        assert_eq!(
            (constraint(),).evaluate_all(&half_assigned),
            HardMediumSoftScore::ZERO
        );
    }
}
