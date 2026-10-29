use crate::planner_domain::{SolverAppliedBlock, SolverPlan, SolverTask};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: a task depending on an already-applied predecessor must start after
/// that applied block ends.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.start_idx.is_some())
        .join((
            SolverPlan::applied_blocks(),
            joiner::less_than(
                |task: &SolverTask| task.start(),
                |block: &SolverAppliedBlock| Some(block.end),
            ),
        ))
        .filter(|task: &SolverTask, block: &SolverAppliedBlock| {
            block.successors.contains(&task.index)
        })
        .penalize(hard_weight(|_: &SolverTask, _: &SolverAppliedBlock| {
            HardMediumSoftScore::of_hard(1)
        }))
        .named("Start after applied predecessor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{applied_block, slots, task};
    use crate::planner_domain::SolverAppliedBlock;
    use solverforge::ConstraintSet;

    fn plan(start_idx: Option<usize>, blocks: Vec<SolverAppliedBlock>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        SolverPlan::new(slots(8), vec![], blocks, vec![], vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_starting_before_the_applied_predecessor_ends() {
        assert_eq!(
            (constraint(),)
                .evaluate_all(&plan(Some(0), vec![applied_block(0, 90, false, vec![0])])),
            HardMediumSoftScore::of_hard(-1)
        );
    }

    #[test]
    fn ignores_unrelated_blocks_and_late_starts() {
        assert_eq!(
            (constraint(),)
                .evaluate_all(&plan(Some(4), vec![applied_block(0, 90, false, vec![0])])),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),)
                .evaluate_all(&plan(Some(0), vec![applied_block(0, 90, false, vec![99])])),
            HardMediumSoftScore::ZERO
        );
    }
}
