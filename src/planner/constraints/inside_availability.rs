use super::support::{clock_contains, task_row, AvailabilityRows, TaskInterval, TimelineRow};
use crate::planner_domain::{SolverAvailability, SolverPlan, SolverSlot, SolverTask};
use chrono::{Datelike, Duration};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: charge one point for every assigned minute. The coverage constraint
/// refunds exactly the minutes contained by the availability union.
pub fn required() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    ConstraintFactory::<SolverPlan, HardMediumSoftScore>::new()
        .for_each(SolverPlan::tasks())
        .filter(|task: &SolverTask| task.start_idx.is_some())
        .penalize(hard_weight(|task: &SolverTask| {
            HardMediumSoftScore::of_hard(task.duration_minutes)
        }))
        .named("Require available minutes")
}

/// HARD: refund assigned minutes covered by canonical disjoint availability
/// facts. A fully covered task nets to zero across the two availability rules.
pub fn covered() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
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
                .for_each(SolverPlan::availability())
                .project(AvailabilityRows),
        )
        .join(joiner::equal(|_: &TimelineRow| ()))
        .filter(|left: &TimelineRow, right: &TimelineRow| covered_minutes(left, right) > 0)
        .reward(hard_weight(|left: &TimelineRow, right: &TimelineRow| {
            HardMediumSoftScore::of_hard(covered_minutes(left, right))
        }))
        .named("Cover available minutes")
}

fn covered_minutes(left: &TimelineRow, right: &TimelineRow) -> i64 {
    match (left, right) {
        (
            TimelineRow::Task(task),
            TimelineRow::Availability {
                weekday,
                start,
                end,
            },
        )
        | (
            TimelineRow::Availability {
                weekday,
                start,
                end,
            },
            TimelineRow::Task(task),
        ) => task_covered_minutes(task, *weekday, *start, *end),
        _ => 0,
    }
}

/// Whether every minute of the interval lies inside the canonical availability
/// union. Shares the scoring rule's per-minute coverage definition so proposal
/// diagnostics can never disagree with the solver about feasibility.
pub(crate) fn fully_covered(interval: &TaskInterval, availability: &[SolverAvailability]) -> bool {
    let duration = (interval.end - interval.start).num_minutes();
    availability
        .iter()
        .map(|window| task_covered_minutes(interval, window.weekday, window.start, window.end))
        .sum::<i64>()
        == duration
}

fn task_covered_minutes(
    task: &TaskInterval,
    weekday: u32,
    window_start: chrono::NaiveTime,
    window_end: chrono::NaiveTime,
) -> i64 {
    let duration = (task.end - task.start).num_minutes();
    (0..duration)
        .filter(|offset| {
            let instant = task.start + Duration::minutes(*offset);
            let local = instant.with_timezone(&task.timezone);
            local.weekday().num_days_from_monday() == weekday
                && clock_contains(local.time(), window_start, window_end)
        })
        .count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use crate::planner_domain::SolverAvailability;
    use chrono::NaiveTime;
    use solverforge::ConstraintSet;

    fn window(id: &str, start: (u32, u32), end: (u32, u32)) -> SolverAvailability {
        SolverAvailability {
            id: id.into(),
            weekday: 0,
            start: NaiveTime::from_hms_opt(start.0, start.1, 0).unwrap(),
            end: NaiveTime::from_hms_opt(end.0, end.1, 0).unwrap(),
        }
    }

    fn plan(start_idx: Option<usize>, windows: Vec<SolverAvailability>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        SolverPlan::new(slots(48), vec![], vec![], windows, vec![], vec![task], 1)
    }

    fn score(plan: &SolverPlan) -> HardMediumSoftScore {
        (required(), covered()).evaluate_all(plan)
    }

    #[test]
    fn penalizes_each_unavailable_minute() {
        assert_eq!(
            score(&plan(Some(0), vec![window("a", (0, 30), (1, 0))])),
            HardMediumSoftScore::of_hard(-30)
        );
    }

    #[test]
    fn combines_adjacent_windows() {
        assert_eq!(
            score(&plan(
                Some(18),
                vec![window("a", (9, 0), (9, 30)), window("b", (9, 30), (10, 0)),],
            )),
            HardMediumSoftScore::ZERO
        );
    }

    #[test]
    fn ignores_unassigned_tasks() {
        assert_eq!(score(&plan(None, Vec::new())), HardMediumSoftScore::ZERO);
    }
}
