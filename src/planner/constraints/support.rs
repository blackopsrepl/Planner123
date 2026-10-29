use crate::planner_domain::SolverTask;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;

#[derive(Clone)]
pub(crate) struct TaskInterval {
    pub(crate) index: usize,
    pub(crate) start: DateTime<Utc>,
    pub(crate) end: DateTime<Utc>,
    pub(super) load: usize,
    pub(super) timezone: Tz,
    pub(super) recovery_minutes: i64,
    pub(crate) excess_high_penalty: i64,
    pub(crate) high_streak_limit: i64,
    /// Ends of applied high-load blocks, attached from the owning task.
    pub(super) applied_predecessor_ends: Vec<DateTime<Utc>>,
}

impl TaskInterval {
    /// The task's interval when it is assigned.
    pub(crate) fn of(task: &SolverTask) -> Option<Self> {
        task.start().map(|start| Self::at(task, start))
    }

    /// The interval the task would occupy starting at `start`; proposal
    /// diagnostics use it to probe candidate slots.
    pub(crate) fn at(task: &SolverTask, start: DateTime<Utc>) -> Self {
        Self {
            index: task.index,
            start,
            end: task.end_from(start),
            load: task.load,
            timezone: task.timezone,
            recovery_minutes: task.recovery_minutes,
            excess_high_penalty: task.excess_high_penalty,
            high_streak_limit: task.high_streak_limit,
            applied_predecessor_ends: task.applied_predecessor_ends.clone(),
        }
    }

    pub(crate) fn overlaps(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        self.start < end && start < self.end
    }

    /// Whether this task carries a high cognitive load (`load` key 2).
    pub(crate) fn is_high(&self) -> bool {
        self.load == 2
    }

    /// Whether a high-load block ending at `predecessor_end` puts this task
    /// under recovery pressure: the block ends at or before the task and
    /// within the task's recovery gap. The single directional definition
    /// shared by the scoring rules and proposal diagnostics.
    pub(crate) fn gap_recovers(
        &self,
        predecessor_end: DateTime<Utc>,
        predecessor_high: bool,
    ) -> bool {
        predecessor_high
            && predecessor_end <= self.start
            && (self.start - predecessor_end).num_minutes() < self.recovery_minutes
    }

    /// How many applied high-load blocks end within this task's recovery gap
    /// before it starts.
    pub(crate) fn applied_recovery_pressure(&self) -> usize {
        self.applied_predecessor_ends
            .iter()
            .filter(|end| self.gap_recovers(**end, true))
            .count()
    }
}
