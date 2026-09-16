use crate::planner_domain::{
    SolverAppliedBlock, SolverAvailability, SolverBusy, SolverCognitiveWindow, SolverSlot,
    SolverTask,
};
use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use solverforge::prelude::*;

#[derive(Clone)]
pub(crate) struct TaskInterval {
    pub(super) index: usize,
    pub(crate) start: DateTime<Utc>,
    pub(crate) end: DateTime<Utc>,
    pub(super) load: usize,
    pub(super) depends_on: Vec<usize>,
    pub(super) timezone: Tz,
    pub(super) recovery_minutes: i64,
    pub(super) excess_high_penalty: i64,
    pub(super) high_streak_limit: i64,
    /// Ends of applied high-load blocks, attached from the owning task.
    pub(super) applied_predecessor_ends: Vec<DateTime<Utc>>,
}

impl TaskInterval {
    pub(crate) fn new(task: &SolverTask, slot: &SolverSlot) -> Self {
        Self {
            index: task.index,
            start: slot.start,
            end: task.end_at(slot),
            load: task.load,
            depends_on: task.depends_on.clone(),
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
    pub(super) fn is_high(&self) -> bool {
        self.load == 2
    }

    /// How many applied high-load blocks end within this task's recovery gap
    /// before it starts.
    pub(super) fn applied_recovery_pressure(&self) -> usize {
        self.applied_predecessor_ends
            .iter()
            .filter(|end| {
                **end <= self.start && (self.start - **end).num_minutes() < self.recovery_minutes
            })
            .count()
    }
}

#[derive(Clone)]
pub(super) enum TimelineRow {
    Task(TaskInterval),
    Busy {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    Applied {
        end: DateTime<Utc>,
        successors: Vec<usize>,
    },
    Availability {
        weekday: u32,
        start: NaiveTime,
        end: NaiveTime,
    },
    Cognitive {
        load: usize,
        start: NaiveTime,
        end: NaiveTime,
        outside_penalty: i64,
    },
}

pub(super) fn task_row(task: &SolverTask, slot: &SolverSlot) -> TimelineRow {
    TimelineRow::Task(TaskInterval::new(task, slot))
}

pub(super) struct BusyRows;

impl Projection<SolverBusy> for BusyRows {
    type Out = TimelineRow;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, busy: &SolverBusy, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(TimelineRow::Busy {
            start: busy.start,
            end: busy.end,
        });
    }
}

pub(super) struct AppliedRows;

impl Projection<SolverAppliedBlock> for AppliedRows {
    type Out = TimelineRow;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, block: &SolverAppliedBlock, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(TimelineRow::Applied {
            end: block.end,
            successors: block.successors.clone(),
        });
    }
}

pub(super) struct AvailabilityRows;

impl Projection<SolverAvailability> for AvailabilityRows {
    type Out = TimelineRow;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, window: &SolverAvailability, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(TimelineRow::Availability {
            weekday: window.weekday,
            start: window.start,
            end: window.end,
        });
    }
}

pub(super) struct CognitiveRows;

impl Projection<SolverCognitiveWindow> for CognitiveRows {
    type Out = TimelineRow;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, window: &SolverCognitiveWindow, sink: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        sink.emit(TimelineRow::Cognitive {
            load: window.load,
            start: window.start,
            end: window.end,
            outside_penalty: window.outside_penalty,
        });
    }
}

pub(super) fn clock_contains(value: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start == end {
        return true;
    }
    if start < end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}
