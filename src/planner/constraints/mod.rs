#![cfg_attr(rustfmt, rustfmt_skip)]
//! Constraint assembly: one rule per file, listed in analysis order.
//!
//! Assignment completeness first, hard feasibility next, soft quality last.

use crate::planner_domain::SolverPlan;
use solverforge::prelude::*;

pub use self::assemble::create_constraints;

pub(crate) use self::assign_priority::scale_assignment_penalties;
pub(crate) use self::cognitive_window::minutes_outside;
pub(crate) use self::high_load_recovery::recovery_penalty;
pub(crate) use self::inside_availability::fully_covered;
pub(crate) use self::support::TaskInterval;

/// Constraint names shared by rule registration and score-analysis lookups so
/// proposal diagnostics can never drift from the scored model.
pub(crate) mod names {
    pub const COGNITIVE_WINDOWS: &str = "Prefer cognitive windows";
    pub const HIGH_LOAD_RECOVERY: &str = "High cognitive-load recovery";
}

mod assign_priority;
mod support;
mod future_only;
mod respect_earliest;
mod respect_hard_deadline;
mod no_busy_overlap;
mod no_task_overlap;
mod dependencies_assigned;
mod dependencies_after_tasks;
mod after_applied_predecessor;
mod inside_availability;
mod soft_deadline;
mod cognitive_window;
mod high_load_recovery;

mod assemble {
    use super::*;

    /// Collects the full scoring model used by `SolverPlan`.
    pub fn create_constraints() -> impl ConstraintSet<SolverPlan, HardMediumSoftScore> {
        (
            assign_priority::constraint(),
            future_only::constraint(),
            respect_earliest::constraint(),
            respect_hard_deadline::constraint(),
            no_busy_overlap::constraint(),
            no_task_overlap::constraint(),
            dependencies_assigned::constraint(),
            dependencies_after_tasks::constraint(),
            after_applied_predecessor::constraint(),
            inside_availability::required(),
            inside_availability::covered(),
            soft_deadline::constraint(),
            cognitive_window::constraint(),
            high_load_recovery::constraint(),
        )
    }
}
