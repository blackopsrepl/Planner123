//! Planning entity: one inbox task that the solver places on the slot grid.
//!
//! `start_idx` is the only planning variable. The remaining time fields are
//! immutable domain data. `horizon_origin` and `slot_minutes` denormalize the
//! slot-grid geometry onto the entity so value-only stream predicates can
//! resolve `start_idx` into a domain instant; a domain test asserts the
//! resolved interval always equals the owning `SolverSlot.start`.

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
    pub slot_minutes: i64,
    pub horizon_origin: DateTime<Utc>,
    pub recovery_minutes: i64,
    pub excess_high_penalty: i64,
    #[planning_variable(value_range_provider = "slots", allows_unassigned = true)]
    pub start_idx: Option<usize>,
}

impl SolverTask {
    /// Resolved start instant, or `None` when unassigned.
    pub fn start(&self) -> Option<DateTime<Utc>> {
        self.start_idx
            .map(|idx| self.horizon_origin + Duration::minutes(idx as i64 * self.slot_minutes))
    }

    /// Resolved end instant, or `None` when unassigned.
    pub fn end(&self) -> Option<DateTime<Utc>> {
        self.start()
            .map(|start| start + Duration::minutes(self.duration_minutes))
    }

    /// Half-open overlap between this task's resolved interval and `[start,end)`.
    pub fn overlaps(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        matches!((self.start(), self.end()), (Some(s), Some(e)) if s < end && start < e)
    }

    /// Signed gap in minutes between this task and `[start,end)`; zero when they
    /// touch or overlap.
    pub fn gap_to(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Option<i64> {
        match (self.start(), self.end()) {
            (Some(s), Some(e)) => {
                if e <= start {
                    Some((start - e).num_minutes())
                } else if end <= s {
                    Some((s - end).num_minutes())
                } else {
                    Some(0)
                }
            }
            _ => None,
        }
    }

    /// Half-open overlap between two resolved task intervals.
    pub fn overlaps_task(&self, other: &SolverTask) -> bool {
        matches!(
            (self.start(), self.end(), other.start(), other.end()),
            (Some(s), Some(e), Some(os), Some(oe)) if s < oe && os < e
        )
    }

    /// Signed gap in minutes between two resolved task intervals; zero when
    /// they touch or overlap, `None` when either is unassigned.
    pub fn gap_between(&self, other: &SolverTask) -> Option<i64> {
        let (a_start, a_end) = (self.start()?, self.end()?);
        let (b_start, b_end) = (other.start()?, other.end()?);
        if a_end <= b_start {
            Some((b_start - a_end).num_minutes())
        } else if b_end <= a_start {
            Some((a_start - b_end).num_minutes())
        } else {
            Some(0)
        }
    }

    /// Whether this task carries a high cognitive load.
    pub fn is_high(&self) -> bool {
        self.load == 2
    }

    /// Local-zone resolved start, used by clock-based preference rules.
    pub fn local_start(&self) -> Option<DateTime<Tz>> {
        self.start()
            .map(|start| start.with_timezone(&self.timezone))
    }

    /// Local-zone resolved end, used by clock-based preference rules.
    pub fn local_end(&self) -> Option<DateTime<Tz>> {
        self.end().map(|end| end.with_timezone(&self.timezone))
    }
}
