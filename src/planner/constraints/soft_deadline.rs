use crate::planner_domain::{SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize minutes that fall past a soft deadline.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| late_minutes(task) > 0)
        .penalize(|task: &SolverTask| HardMediumSoftScore::of_soft(late_minutes(task)))
        .named("Prefer soft deadlines")
}

fn late_minutes(task: &SolverTask) -> i64 {
    match (task.end(), task.soft_deadline) {
        (Some(end), Some(deadline)) if end > deadline => (end - deadline).num_minutes(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use chrono::Duration;
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, deadline_offset_minutes: i64) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        task.soft_deadline = Some(task.horizon_origin + Duration::minutes(deadline_offset_minutes));
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
