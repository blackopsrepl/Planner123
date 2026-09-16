use super::support::TaskInterval;
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: charge each high-load task once when preceding inbox and applied
/// high-load work within its recovery gap reaches the configured streak limit.
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
        .project(|task: &SolverTask, slot: &SolverSlot| TaskInterval::new(task, slot))
        .filter(|interval: &TaskInterval| interval.is_high())
        .group_by(
            |_: &TaskInterval| (),
            collect_vec(|interval: &TaskInterval| interval.clone()),
        )
        .penalize(|_: &(), intervals: &CollectedVec<TaskInterval>| {
            HardMediumSoftScore::of_soft(
                intervals
                    .iter()
                    .map(|target| recovery_penalty(target, intervals.iter()))
                    .sum(),
            )
        })
        .named(super::names::HIGH_LOAD_RECOVERY)
}

/// Target-owned recovery penalty shared by scoring and proposal diagnostics.
pub(crate) fn recovery_penalty<'a>(
    target: &TaskInterval,
    intervals: impl IntoIterator<Item = &'a TaskInterval>,
) -> i64 {
    if !target.is_high() {
        return 0;
    }
    let inbox_pressure = intervals
        .into_iter()
        .filter(|predecessor| {
            predecessor.index != target.index
                && target.gap_recovers(predecessor.end, predecessor.is_high())
        })
        .count();
    let pressure = inbox_pressure + target.applied_recovery_pressure();
    if pressure >= target.high_streak_limit as usize {
        target.excess_high_penalty
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};

    fn plan(start_idxs: &[Option<usize>], streak_limit: i64) -> SolverPlan {
        let tasks = start_idxs
            .iter()
            .enumerate()
            .map(|(index, start_idx)| {
                let mut value = task(3, 60);
                value.id = index;
                value.task_id = format!("task-{index}");
                value.index = index;
                value.load = 2;
                value.recovery_minutes = 150;
                value.high_streak_limit = streak_limit;
                value.start_idx = *start_idx;
                value
            })
            .collect();
        SolverPlan::new(slots(8), vec![], vec![], vec![], vec![], tasks, 1)
    }

    #[test]
    fn penalizes_a_target_once_when_inbox_pressure_reaches_the_limit() {
        let plan = plan(&[Some(0), Some(2), Some(4)], 2);
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::of_soft(-5)
        );
    }

    #[test]
    fn pressure_below_the_limit_is_ignored() {
        let separated = plan(&[Some(0), Some(2)], 2);
        assert_eq!(
            (constraint(),).evaluate_all(&separated),
            HardMediumSoftScore::ZERO
        );
    }

    #[test]
    fn inbox_and_applied_pressure_combine_at_the_threshold() {
        let mut plan = plan(&[Some(0), Some(2)], 2);
        plan.tasks[1].applied_predecessor_ends = vec![slots(1)[0].start];
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::of_soft(-5)
        );
    }

    #[test]
    fn low_load_targets_and_unassigned_predecessors_are_ignored() {
        let mut plan = plan(&[None, Some(2)], 1);
        plan.tasks[1].load = 1;
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::ZERO
        );
    }
}
