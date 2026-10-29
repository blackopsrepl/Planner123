//! Planning entity: one inbox task that the solver places on the slot grid.
//!
//! `start_idx` is the only planning variable: an index into the uniform slot
//! grid `SolverPlan.slots`. The task carries the grid geometry, so its own
//! interval is known from the entity alone and constraint streams can index
//! it directly instead of joining every rule through the slot collection.

use chrono::{DateTime, Duration, Utc};
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
    /// Medium assignment penalty derived from priority and scaled above all
    /// possible soft-deadline lateness in this solve.
    pub priority_weight: i64,
    /// Load-level key: 0 low, 1 medium, 2 high.
    pub load: usize,
    pub earliest_at: Option<DateTime<Utc>>,
    pub hard_deadline: Option<DateTime<Utc>>,
    pub soft_deadline: Option<DateTime<Utc>>,
    /// Indexes of predecessor tasks this task must start after.
    pub depends_on: Vec<usize>,
    /// Ends of applied high cognitive-load blocks this task must recover
    /// after. Relational data, attached by the loader.
    pub applied_predecessor_ends: Vec<DateTime<Utc>>,
    /// Global lower bound: optimization time, so no assignment lands in the
    /// past.
    pub not_before: DateTime<Utc>,
    pub timezone: Tz,
    pub recovery_minutes: i64,
    pub excess_high_penalty: i64,
    /// Number of preceding high-load blocks that triggers one excess recovery
    /// penalty for this task.
    pub high_streak_limit: i64,
    /// Start of slot 0 of the grid `start_idx` indexes.
    pub grid_origin: DateTime<Utc>,
    /// Width of one grid slot.
    pub slot_minutes: i64,
    #[planning_variable(value_range_provider = "slots", allows_unassigned = true)]
    pub start_idx: Option<usize>,
}

impl SolverTask {
    /// Start instant of grid slot `index`. The loader builds `SolverPlan.slots`
    /// from the same origin and width, so this equals `slots[index].start`.
    pub fn slot_start(&self, index: usize) -> DateTime<Utc> {
        self.grid_origin + Duration::minutes(index as i64 * self.slot_minutes)
    }

    /// End instant when this task starts at `start`.
    pub fn end_from(&self, start: DateTime<Utc>) -> DateTime<Utc> {
        start + Duration::minutes(self.duration_minutes)
    }

    /// Assigned start, or `None` while unassigned.
    pub fn start(&self) -> Option<DateTime<Utc>> {
        self.start_idx.map(|index| self.slot_start(index))
    }

    /// Assigned end, or `None` while unassigned. With `start` this is the
    /// task's half-open interval `[start, end)`; every rule derives it here.
    pub fn end(&self) -> Option<DateTime<Utc>> {
        self.start().map(|start| self.end_from(start))
    }
}
