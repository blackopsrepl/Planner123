use super::clock::minutes_in_window;
use super::support::TaskInterval;
use crate::planner_domain::{SolverCognitiveWindow, SolverPlan, SolverTask};
use chrono::NaiveTime;
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize minutes scheduled outside the cognitive window that owns the
/// task's load level.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.start_idx.is_some())
        .join((
            SolverPlan::cognitive_windows(),
            joiner::equal_bi(
                |task: &SolverTask| task.load,
                |window: &SolverCognitiveWindow| window.load,
            ),
        ))
        .filter(|task: &SolverTask, window: &SolverCognitiveWindow| penalty(task, window) > 0)
        .penalize(|task: &SolverTask, window: &SolverCognitiveWindow| {
            HardMediumSoftScore::of_soft(penalty(task, window))
        })
        .named(super::names::COGNITIVE_WINDOWS)
}

fn penalty(task: &SolverTask, window: &SolverCognitiveWindow) -> i64 {
    match TaskInterval::of(task) {
        Some(interval) if window.outside_penalty > 0 => {
            minutes_outside(&interval, window.start, window.end) * window.outside_penalty
        }
        _ => 0,
    }
}

/// Counts task minutes whose local clock time falls outside the daily window,
/// including across midnight. Shared with proposal diagnostics.
pub(crate) fn minutes_outside(
    task: &TaskInterval,
    window_start: NaiveTime,
    window_end: NaiveTime,
) -> i64 {
    let duration = (task.end - task.start).num_minutes();
    duration
        - minutes_in_window(
            task.start,
            task.end,
            task.timezone,
            None,
            window_start,
            window_end,
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use crate::planner_domain::SolverCognitiveWindow;
    use solverforge::ConstraintSet;

    fn window(start: (u32, u32), end: (u32, u32), penalty: i64) -> SolverCognitiveWindow {
        SolverCognitiveWindow {
            id: "window-1".into(),
            load: 1,
            start: NaiveTime::from_hms_opt(start.0, start.1, 0).unwrap(),
            end: NaiveTime::from_hms_opt(end.0, end.1, 0).unwrap(),
            outside_penalty: penalty,
        }
    }

    fn plan(start_idx: Option<usize>, windows: Vec<SolverCognitiveWindow>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        SolverPlan::new(slots(24), vec![], vec![], vec![], windows, vec![task], 1)
    }

    #[test]
    fn penalizes_minutes_outside_the_window() {
        let score = (constraint(),).evaluate_all(&plan(Some(0), vec![window((10, 0), (12, 0), 2)]));
        assert_eq!(score, HardMediumSoftScore::of_soft(-120));
    }

    #[test]
    fn counts_minutes_outside_the_window_across_midnight() {
        let mut task = task(3, 120);
        task.start_idx = Some(46);
        let plan = SolverPlan::new(
            slots(48),
            vec![],
            vec![],
            vec![],
            vec![window((10, 0), (12, 0), 2)],
            vec![task],
            1,
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan),
            HardMediumSoftScore::of_soft(-240)
        );
    }

    #[test]
    fn ignores_inside_or_unassigned_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(Some(20), vec![window((10, 0), (12, 0), 2)])),
            HardMediumSoftScore::ZERO
        );
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None, vec![window((10, 0), (12, 0), 2)])),
            HardMediumSoftScore::ZERO
        );
    }
}
