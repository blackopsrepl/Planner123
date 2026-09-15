use crate::planner_domain::{SolverBusy, SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize high cognitive-load tasks placed too soon after an applied
/// high cognitive-load block.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .join((
            ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
                .for_each(SolverPlan::busy()),
            |task: &SolverTask, busy: &SolverBusy| {
                task.is_high()
                    && busy.high
                    && matches!(gap_to_block(task, busy), Some(gap) if gap < task.recovery_minutes)
            },
        ))
        .penalize(|task: &SolverTask, _busy: &SolverBusy| {
            HardMediumSoftScore::of_soft(task.excess_high_penalty)
        })
        .named("Applied high cognitive-load recovery")
}

fn gap_to_block(task: &SolverTask, busy: &SolverBusy) -> Option<i64> {
    let (start, end) = (task.start()?, task.end()?);
    if end <= busy.start {
        Some((busy.start - end).num_minutes())
    } else if busy.end <= start {
        Some((start - busy.end).num_minutes())
    } else {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn block(start_offset: i64, end_offset: i64, high: bool) -> SolverBusy {
        SolverBusy {
            id: "block-0".into(),
            index: 0,
            start: origin() + Duration::minutes(start_offset),
            end: origin() + Duration::minutes(end_offset),
            high,
            successors: Vec::new(),
            event_id: "event-0".into(),
            event_title: "Applied".into(),
            calendar_id: "cal".into(),
            recurring: false,
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
