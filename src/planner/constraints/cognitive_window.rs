use super::support::{clock_contains, task_row, CognitiveRows, TaskInterval, TimelineRow};
use crate::planner_domain::{SolverPlan, SolverSlot, SolverTask};
use chrono::{Duration, NaiveTime};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: penalize minutes scheduled outside the cognitive window that owns the
/// task's load level.
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
                .for_each(SolverPlan::cognitive_windows())
                .project(CognitiveRows),
        )
        .join(joiner::equal(|_: &TimelineRow| ()))
        .filter(|left: &TimelineRow, right: &TimelineRow| penalty(left, right) > 0)
        .penalize(|left: &TimelineRow, right: &TimelineRow| {
            HardMediumSoftScore::of_soft(penalty(left, right))
        })
        .named("Prefer cognitive windows")
}

fn penalty(left: &TimelineRow, right: &TimelineRow) -> i64 {
    match (left, right) {
        (
            TimelineRow::Task(task),
            TimelineRow::Cognitive {
                load,
                start,
                end,
                outside_penalty,
            },
        )
        | (
            TimelineRow::Cognitive {
                load,
                start,
                end,
                outside_penalty,
            },
            TimelineRow::Task(task),
        ) if task.load == *load && *outside_penalty > 0 => {
            minutes_outside(task, *start, *end) * *outside_penalty
        }
        _ => 0,
    }
}

/// Counts task minutes whose local clock time falls outside the daily window,
/// including across midnight.
fn minutes_outside(task: &TaskInterval, window_start: NaiveTime, window_end: NaiveTime) -> i64 {
    let mut cursor = task.start.with_timezone(&task.timezone);
    let end = task.end.with_timezone(&task.timezone);
    let mut outside = 0;
    while cursor < end {
        if !clock_contains(cursor.time(), window_start, window_end) {
            outside += 1;
        }
        cursor += Duration::minutes(1);
    }
    outside
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
        SolverPlan::new(slots(24), vec![], vec![], windows, vec![task], 1)
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
