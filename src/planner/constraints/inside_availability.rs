use crate::planner_domain::{SolverAvailability, SolverPlan, SolverTask};
use chrono::{Datelike, Duration};
use solverforge::prelude::*;
use solverforge::{IncrementalConstraint, IncrementalConstraintSealed};
use solverforge_core::ConstraintRef;

/// HARD: every assigned minute must fall inside some weekly availability
/// window.
///
/// Coverage needs "no window contains this minute", a negated existence the
/// fluent keyed-existence API cannot express, so this rule is a small custom
/// `IncrementalConstraint` over the raw availability facts. Availability stays
/// data; the rule lives here and nowhere else.
pub fn constraint() -> impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> {
    AvailabilityConstraint::new()
}

struct AvailabilityConstraint {
    constraint_ref: ConstraintRef,
    last_score: HardMediumSoftScore,
}

impl AvailabilityConstraint {
    fn new() -> Self {
        Self {
            constraint_ref: ConstraintRef::new("", "Inside availability"),
            last_score: HardMediumSoftScore::ZERO,
        }
    }

    fn score_for(plan: &SolverPlan) -> HardMediumSoftScore {
        HardMediumSoftScore::of_hard(-(violations(plan) as i64))
    }
}

fn violations(plan: &SolverPlan) -> usize {
    plan.tasks
        .iter()
        .filter(|task| violates(task, &plan.availability))
        .count()
}

fn violates(task: &SolverTask, windows: &[SolverAvailability]) -> bool {
    let (Some(start), Some(end)) = (task.local_start(), task.local_end()) else {
        return false;
    };
    if start >= end {
        return true;
    }
    let mut cursor = start;
    while cursor < end {
        let weekday = cursor.weekday().num_days_from_monday();
        let covered = windows.iter().any(|window| {
            window.weekday == weekday && clock_contains(cursor.time(), window.start, window.end)
        });
        if !covered {
            return true;
        }
        cursor += Duration::minutes(1);
    }
    false
}

fn clock_contains(
    value: chrono::NaiveTime,
    start: chrono::NaiveTime,
    end: chrono::NaiveTime,
) -> bool {
    if start == end {
        return true;
    }
    if start < end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}

impl IncrementalConstraintSealed for AvailabilityConstraint {}

impl IncrementalConstraint<SolverPlan, HardMediumSoftScore> for AvailabilityConstraint {
    fn evaluate(&self, plan: &SolverPlan) -> HardMediumSoftScore {
        Self::score_for(plan)
    }

    fn match_count(&self, plan: &SolverPlan) -> usize {
        violations(plan)
    }

    fn initialize(&mut self, plan: &SolverPlan) -> HardMediumSoftScore {
        self.last_score = Self::score_for(plan);
        self.last_score
    }

    fn on_insert(
        &mut self,
        plan: &SolverPlan,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> HardMediumSoftScore {
        let next = Self::score_for(plan);
        let delta = next - self.last_score;
        self.last_score = next;
        delta
    }

    fn on_retract(
        &mut self,
        _plan: &SolverPlan,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> HardMediumSoftScore {
        HardMediumSoftScore::ZERO
    }

    fn reset(&mut self) {
        self.last_score = HardMediumSoftScore::ZERO;
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }

    fn is_hard(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner_domain::test_support::{slots, task};
    use chrono::NaiveTime;
    use solverforge::ConstraintSet;

    fn workweek() -> Vec<SolverAvailability> {
        (0..7)
            .map(|weekday| SolverAvailability {
                id: format!("day-{weekday}"),
                index: weekday as usize,
                weekday,
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            })
            .collect()
    }

    fn plan(start_idx: Option<usize>) -> SolverPlan {
        let mut task = task(3, 60);
        task.start_idx = start_idx;
        SolverPlan::new(slots(48), vec![], workweek(), vec![], vec![task], 1)
    }

    #[test]
    fn penalizes_assignment_outside_availability() {
        let score = (constraint(),).evaluate_all(&plan(Some(0)));
        assert_eq!(score, HardMediumSoftScore::of_hard(-1));
    }

    #[test]
    fn allows_assignment_inside_availability() {
        // 2026-01-05 is a Monday; slot 18 starts at 09:00 UTC.
        let score = (constraint(),).evaluate_all(&plan(Some(18)));
        assert_eq!(score, HardMediumSoftScore::ZERO);
    }

    #[test]
    fn ignores_unassigned_tasks() {
        assert_eq!(
            (constraint(),).evaluate_all(&plan(None)),
            HardMediumSoftScore::ZERO
        );
    }
}
