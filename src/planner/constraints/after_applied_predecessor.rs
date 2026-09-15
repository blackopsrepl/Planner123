use crate::planner_domain::{SolverBusy, SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task depending on an already-applied predecessor must start after
/// that applied block ends.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::busy()),
            |task: &SolverTask, busy: &SolverBusy| {
                busy.successors.contains(&task.index)
                    && task.start().is_some_and(|start| start < busy.end)
            },
        ))
        .penalize(hard_weight(|_: &SolverTask, _: &SolverBusy| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("Start after applied predecessor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn applied(start_offset: i64, end_offset: i64, successors: Vec<usize>) -> SolverBusy {
        SolverBusy {
            id: "applied-0".into(),
            index: 0,
            start: origin() + Duration::minutes(start_offset),
            end: origin() + Duration::minutes(end_offset),
            high: false,
            successors,
            event_id: "event-0".into(),
            event_title: "Applied".into(),
            calendar_id: "cal".into(),
            recurring: false,
        }
    }

    fn plan(start_idx: Option<usize>, blocks: Vec<SolverBusy>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        SolverPlan::new(slots(8), blocks, vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_starting_before_the_applied_predecessor_ends() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), vec![applied(0, 90, vec![0])])),
            HardMediumSoftScore::of_hard(-1)
        );
    }

    #[test]
    fn ignores_unrelated_blocks_and_late_starts() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(4), vec![applied(0, 90, vec![0])])),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), vec![applied(0, 90, vec![99])])),
            HardMediumSoftScore::ZERO
        );
    }
}
