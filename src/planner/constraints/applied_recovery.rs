use super::support::{task_row, BusyRows, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize high cognitive-load tasks placed too soon after an applied
/// high cognitive-load block.
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
        .filter(|left: &TimelineRow, right: &TimelineRow| violates(left, right))
        .penalize(|left: &TimelineRow, right: &TimelineRow| {
            HardMediumSoftScore::of_soft(penalty(left, right))
        })
        .named("Applied high cognitive-load recovery")
}

fn violates(left: &TimelineRow, right: &TimelineRow) -> bool {
    match (left, right) {
        (
            TimelineRow::Task(task),
            TimelineRow::Busy {
                start, end, high, ..
            },
        )
        | (
            TimelineRow::Busy {
                start, end, high, ..
            },
            TimelineRow::Task(task),
        ) => task.is_high() && *high && task.gap(*start, *end) < task.recovery_minutes,
        _ => false,
    }
}

fn penalty(left: &TimelineRow, right: &TimelineRow) -> i64 {
    match (left, right) {
        (TimelineRow::Task(task), TimelineRow::Busy { .. })
        | (TimelineRow::Busy { .. }, TimelineRow::Task(task)) => task.excess_high_penalty,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use crate::planner_domain::SolverBusy;
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn block(start_offset: i64, end_offset: i64, high: bool) -> SolverBusy {
        SolverBusy {
            id: "block-0".into(),
            start: origin() + Duration::minutes(start_offset),
            end: origin() + Duration::minutes(end_offset),
            high,
            successors: Vec::new(),
        }
    }

    fn plan(load: usize, start_idx: Option<usize>, blocks: Vec<SolverBusy>) -> SolverPlan {
        let mut task = task(3, 60);
        task.load = load;
        task.start_idx = start_idx;
        SolverPlan::new(slots(8), blocks, vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_high_load_task_too_soon_after_applied_block() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(2, Some(2), vec![block(0, 60, true)])),
            HardMediumSoftScore::of_soft(-5)
        );
    }

    #[test]
    fn ignores_low_load_and_non_high_blocks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(1, Some(2), vec![block(0, 60, true)])),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(2, Some(2), vec![block(0, 60, false)])),
            HardMediumSoftScore::ZERO
        );
    }
}
