use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// MEDIUM: penalize minutes that fall past a soft deadline.
///
/// "Soft" describes the deadline's feasibility, not its score level: lateness
/// must still dominate the soft cognitive and recovery preferences.
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
            HardMediumSoftScore::of_medium(late_minutes(task, slot))
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
    use crate::planner_domain::SolverCognitiveWindow;
    use chrono::{Duration, NaiveTime};
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, deadline_offset_minutes: i64) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        task.soft_deadline = Some(origin() + Duration::minutes(deadline_offset_minutes));
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_lateness_proportionally() {
        let score = (constraint(),).evaluate_all(&plan(Some(4), 120));
        assert_eq!(score, HardMediumSoftScore::of_medium(-60));
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

    #[test]
    fn deadline_lateness_dominates_soft_cognitive_preferences() {
        use crate::planner::constraints::cognitive_window;

        // Slot 0 finishes on time but sits outside the low-load preference
        // window; slot 4 sits inside it but ends an hour past the deadline.
        let window = SolverCognitiveWindow {
            id: "window-1".into(),
            load: 1,
            start: NaiveTime::from_hms_opt(0, 30, 0).unwrap(),
            end: NaiveTime::from_hms_opt(2, 0, 0).unwrap(),
            outside_penalty: 2,
        };
        let mut on_time = plan(Some(0), 120);
        on_time.cognitive_windows = vec![window.clone()];
        let mut late = plan(Some(4), 120);
        late.cognitive_windows = vec![window];

        let on_time_score = (constraint(), cognitive_window::constraint()).evaluate_all(&on_time);
        let late_score = (constraint(), cognitive_window::constraint()).evaluate_all(&late);
        assert_eq!(on_time_score, HardMediumSoftScore::of(0, 0, -60));
        assert_eq!(late_score, HardMediumSoftScore::of(0, -60, -120));
        assert!(on_time_score > late_score, "on-time placement must win");
    }
}
