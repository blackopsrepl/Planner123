use crate::planner_domain::{
    SolverAvailability, SolverBusy, SolverCognitiveWindow, SolverSlot, SolverTask,
};
use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use solverforge::prelude::*;

#[derive(Clone)]
pub(super) struct TaskInterval {
    pub index: usize,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub load: usize,
    pub depends_on: Vec<usize>,
    pub timezone: Tz,
    pub recovery_minutes: i64,
    pub excess_high_penalty: i64,
}

impl TaskInterval {
    pub fn new(task: &SolverTask, slot: &SolverSlot) -> Self {
        Self {
            index: task.index,
            start: slot.start,
            end: task.end_at(slot),
            load: task.load,
            depends_on: task.depends_on.clone(),
            timezone: task.timezone,
            recovery_minutes: task.recovery_minutes,
            excess_high_penalty: task.excess_high_penalty,
        }
    }

    pub fn overlaps(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        self.start < end && start < self.end
    }

    pub fn gap(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
        if self.end <= start {
            (start - self.end).num_minutes()
        } else if end <= self.start {
            (self.start - end).num_minutes()
        } else {
            0
        }
    }

    /// Whether this task carries a high cognitive load (`load` key 2).
    pub fn is_high(&self) -> bool {
        self.load == 2
    }
}

#[derive(Clone)]
pub(super) enum TimelineRow {
    Task(TaskInterval),
    Busy {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        high: bool,
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
            high: busy.high,
            successors: busy.successors.clone(),
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
