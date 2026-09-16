use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize minutes that fall past a soft deadline.
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
        .filter(|task: &SolverTask, slot: &SolverSlot| late_minutes(task, slot) > 0)
        .penalize(|task: &SolverTask, slot: &SolverSlot| {
            HardMediumSoftScore::of_soft(late_minutes(task, slot))
        })
        .named("Prefer soft deadlines")
}

fn late_minutes(task: &SolverTask, slot: &SolverSlot) -> i64 {
    let end = task.end_at(slot);
    match task.soft_deadline {
        Some(deadline) if end > deadline => (end - deadline).num_minutes(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, deadline_offset_minutes: i64) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        task.soft_deadline = Some(origin() + Duration::minutes(deadline_offset_minutes));
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_lateness_proportionally() {
        let score = (constraint(),).evaluate_all(&plan(Some(4), 120));
        assert_eq!(score, HardMediumSoftScore::of_soft(-60));
    }

    #[test]
    fn ignores_on_time_or_unassigned_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(0), 60)),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None, 60)),
            HardMediumSoftScore::ZERO
        );
    }
}
