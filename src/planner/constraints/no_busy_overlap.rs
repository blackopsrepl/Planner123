use crate::planner_domain::{SolverBusy, SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task must not overlap any existing event or applied block.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::busy()),
            |task: &SolverTask, busy: &SolverBusy| task.overlaps(busy.start, busy.end),
        ))
        .penalize(hard_weight(|_: &SolverTask, _: &SolverBusy| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("No overlap with existing busy time")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn busy(start_offset: i64, end_offset: i64) -> SolverBusy {
        SolverBusy {
            id: "busy-0".into(),
            index: 0,
            start: origin() + Duration::minutes(start_offset),
            end: origin() + Duration::minutes(end_offset),
            high: false,
            successors: Vec::new(),
            event_id: "event-0".into(),
            event_title: "Existing".into(),
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
    fn penalizes_overlap_with_busy_time() {
        let score = (constraint(),).evaluate_all(&plan(Some(1), vec![busy(30, 90)]));
        assert_eq!(score, HardMediumSoftScore::of_hard(-1));
    }

    #[test]
    fn touching_but_non_overlapping_is_allowed() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), vec![busy(0, 60)])),
            HardMediumSoftScore::ZERO
        );
    }

    #[test]
    fn ignores_unassigned_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None, vec![busy(0, 240)])),
            HardMediumSoftScore::ZERO
        );
    }
}
