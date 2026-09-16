use super::support::{task_row, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: charge a high-load task once when the number of applied high-load
/// blocks ending within its recovery gap before it starts reaches the
/// configured streak limit. Blocks that start after the task never count.
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
        .filter(|row: &TimelineRow| violation_penalty(row) > 0)
        .penalize(|row: &TimelineRow| HardMediumSoftScore::of_soft(violation_penalty(row)))
        .named(super::names::APPLIED_RECOVERY)
}

/// The excess penalty a row's task owes for violating applied recovery, or
/// zero when it is not a high-load task within the streak limit.
fn violation_penalty(row: &TimelineRow) -> i64 {
    match row {
        TimelineRow::Task(interval)
            if interval.is_high()
                && interval.high_streak_limit <= interval.applied_recovery_pressure() as i64 =>
        {
            interval.excess_high_penalty
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{origin, slots, task};
    use chrono::Duration;

    /// Attaches `count` applied high-load block ends before `origin`, then
    /// places the task two slots (one hour) into the grid.
    fn plan(start_idx: Option<usize>, streak_limit: i64, ends: usize) -> SolverPlan {
        let mut task = task(3, 60);
        task.load = 2;
        task.recovery_minutes = 120;
        task.start_idx = start_idx;
        task.high_streak_limit = streak_limit;
        task.applied_predecessor_ends = (0..ends)
            .map(|index| origin() - Duration::minutes(index as i64 * 30))
            .collect();
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_when_applied_blocks_reach_the_streak_limit() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), 1, 1)),
            HardMediumSoftScore::of_soft(-5)
        );
    }

    #[test]
    fn a_single_block_stays_within_a_limit_of_two() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), 2, 1)),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(2), 2, 2)),
            HardMediumSoftScore::of_soft(-5)
        );
    }

    #[test]
    fn blocks_ending_after_the_task_never_count() {
        let mut task = task(3, 60);
        task.load = 2;
        task.recovery_minutes = 120;
        task.start_idx = Some(0);
        // Block ends at origin + 120, well after the task ends at origin + 60.
        task.applied_predecessor_ends = vec![origin() + Duration::minutes(120)];
        let plan = SolverPlan::new(slots(8), vec![], vec![], vec![], vec![], vec![task], 1);
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::ZERO
        );
    }

    #[test]
    fn low_load_tasks_are_ignored() {
        let mut task = task(3, 60);
        task.load = 1;
        task.recovery_minutes = 120;
        task.start_idx = Some(2);
        task.applied_predecessor_ends = vec![origin()];
        let plan = SolverPlan::new(slots(8), vec![], vec![], vec![], vec![], vec![task], 1);
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::ZERO
        );
    }
}
