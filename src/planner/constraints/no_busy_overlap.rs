use super::support::{task_row, BusyRows, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task must not overlap any existing event or applied block.
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
        .merge(
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::busy())
                .project(BusyRows),
        )
        .join(joiner::equal(|_: &TimelineRow| ()))
        .filter(|left: &TimelineRow, right: &TimelineRow| task_overlaps_busy(left, right))
        .penalize(hard_weight(|_: &TimelineRow, _: &TimelineRow| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("No overlap with existing busy time")
}

fn task_overlaps_busy(left: &TimelineRow, right: &TimelineRow) -> bool {
    match (left, right) {
        (TimelineRow::Task(task), TimelineRow::Busy { start, end, .. })
        | (TimelineRow::Busy { start, end, .. }, TimelineRow::Task(task)) => {
            task.overlaps(*start, *end)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use crate::planner_domain::SolverBusy;
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn busy(start_offset: i64, end_offset: i64) -> SolverBusy {
        SolverBusy {
            id: "busy-0".into(),
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
