//! Planning entity: one inbox task that the solver places on the slot grid.
//!
//! `start_idx` is the only planning variable. Its value is resolved through
//! the owning `SolverPlan.slots` collection by constraint streams.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use solverforge::prelude::*;

/// A single task whose start instant the solver chooses.
#[planning_entity]
pub struct SolverTask {
    #[planning_id]
    pub id: usize,
    pub task_id: String,
    pub index: usize,
    pub duration_minutes: i64,
    /// Priority weight from planner settings; scales the medium assignment
    /// penalty so high-priority tasks are scheduled first.
    pub priority_weight: i64,
    /// Load-level key: 0 low, 1 medium, 2 high.
    pub load: usize,
    pub earliest_at: Option<DateTime<Utc>>,
    pub hard_deadline: Option<DateTime<Utc>>,
    pub soft_deadline: Option<DateTime<Utc>>,
    /// Indexes of predecessor tasks this task must start after.
    pub depends_on: Vec<usize>,
    /// Global lower bound: optimization time, so no assignment lands in the
    /// past.
    pub not_before: DateTime<Utc>,
    pub timezone: Tz,
    pub recovery_minutes: i64,
    pub excess_high_penalty: i64,
    #[planning_variable(value_range_provider = "slots", allows_unassigned = true)]
    pub start_idx: Option<usize>,
}
